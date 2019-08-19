use std::path::Path;

use futures::compat::{Future01CompatExt, Stream01CompatExt, AsyncWrite01CompatExt};
use futures::io::{self, AsyncRead, AsyncReadExt as _, AsyncWrite};
use futures::future::Future;
use futures::stream::TryStreamExt;

use super::data_stream::DataStream;

use crate::http::hyper;

use crate::ext::AsyncReadExt;

/// The number of bytes to read into the "peek" buffer.
const PEEK_BYTES: usize = 512;

/// Type representing the data in the body of an incoming request.
///
/// This type is the only means by which the body of a request can be retrieved.
/// This type is not usually used directly. Instead, types that implement
/// [`FromData`](crate::data::FromData) are used via code generation by
/// specifying the `data = "<var>"` route parameter as follows:
///
/// ```rust
/// # #![feature(proc_macro_hygiene, async_await)]
/// # #[macro_use] extern crate rocket;
/// # type DataGuard = rocket::data::Data;
/// #[post("/submit", data = "<var>")]
/// fn submit(var: DataGuard) { /* ... */ }
/// # fn main() { }
/// ```
///
/// Above, `DataGuard` can be any type that implements `FromData`. Note that
/// `Data` itself implements `FromData`.
///
/// # Reading Data
///
/// Data may be read from a `Data` object by calling either the
/// [`open()`](Data::open()) or [`peek()`](Data::peek()) methods.
///
/// The `open` method consumes the `Data` object and returns the raw data
/// stream. The `Data` object is consumed for safety reasons: consuming the
/// object ensures that holding a `Data` object means that all of the data is
/// available for reading.
///
/// The `peek` method returns a slice containing at most 512 bytes of buffered
/// body data. This enables partially or fully reading from a `Data` object
/// without consuming the `Data` object.
pub struct Data {
    buffer: Vec<u8>,
    is_complete: bool,
    stream: Box<dyn AsyncRead + Unpin + Send>,
}

impl Data {
    /// Returns the raw data stream.
    ///
    /// The stream contains all of the data in the body of the request,
    /// including that in the `peek` buffer. The method consumes the `Data`
    /// instance. This ensures that a `Data` type _always_ represents _all_ of
    /// the data in a request.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Data;
    ///
    /// fn handler(data: Data) {
    ///     let stream = data.open();
    /// }
    /// ```
    pub fn open(mut self) -> DataStream {
        let buffer = std::mem::replace(&mut self.buffer, vec![]);
        let stream = std::mem::replace(&mut self.stream, Box::new(&[][..]));
        DataStream(buffer, stream)
    }

    pub(crate) fn from_hyp(body: hyper::Body) -> impl Future<Output = Data> {
        // TODO.async: This used to also set the read timeout to 5 seconds.

        Data::new(body)
    }

    /// Retrieve the `peek` buffer.
    ///
    /// The peek buffer contains at most 512 bytes of the body of the request.
    /// The actual size of the returned buffer varies by web request. The
    /// [`peek_complete`](#method.peek_complete) method can be used to determine
    /// if this buffer contains _all_ of the data in the body of the request.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Data;
    ///
    /// fn handler(data: Data) {
    ///     let peek = data.peek();
    /// }
    /// ```
    #[inline(always)]
    pub fn peek(&self) -> &[u8] {
        if self.buffer.len() > PEEK_BYTES {
            &self.buffer[..PEEK_BYTES]
        } else {
            &self.buffer
        }
    }

    /// Returns true if the `peek` buffer contains all of the data in the body
    /// of the request. Returns `false` if it does not or if it is not known if
    /// it does.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Data;
    ///
    /// fn handler(data: Data) {
    ///     if data.peek_complete() {
    ///         println!("All of the data: {:?}", data.peek());
    ///     }
    /// }
    /// ```
    #[inline(always)]
    pub fn peek_complete(&self) -> bool {
        self.is_complete
    }

    /// A helper method to write the body of the request to any `Write` type.
    ///
    /// This method is identical to `io::copy(&mut data.open(), writer)`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::io;
    /// use rocket::Data;
    ///
    /// fn handler(mut data: Data) -> io::Result<String> {
    ///     // write all of the data to stdout
    ///     data.stream_to(&mut io::stdout())
    ///         .map(|n| format!("Wrote {} bytes.", n))
    /// }
    /// ```
    #[inline(always)]
    pub fn stream_to<'w, W: AsyncWrite + Unpin>(self, writer: &'w mut W) -> impl Future<Output = io::Result<u64>> + 'w {
        Box::pin(async move {
            let stream = self.open();
            stream.copy_into(writer).await
        })
    }

    /// A helper method to write the body of the request to a file at the path
    /// determined by `path`.
    ///
    /// This method is identical to
    /// `io::copy(&mut self.open(), &mut File::create(path)?)`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::io;
    /// use rocket::Data;
    ///
    /// fn handler(mut data: Data) -> io::Result<String> {
    ///     data.stream_to_file("/static/file")
    ///         .map(|n| format!("Wrote {} bytes to /static/file", n))
    /// }
    /// ```
    #[inline(always)]
    pub fn stream_to_file<P: AsRef<Path> + Send + 'static>(self, path: P) -> impl Future<Output = io::Result<u64>> {
        Box::pin(async move {
            let mut file = tokio::fs::File::create(path).compat().await?.compat();
            self.stream_to(&mut file).await
        })
    }

    // Creates a new data object with an internal buffer `buf`, where the cursor
    // in the buffer is at `pos` and the buffer has `cap` valid bytes. Thus, the
    // bytes `vec[pos..cap]` are buffered and unread. The remainder of the data
    // bytes can be read from `stream`.
    #[inline(always)]
    pub(crate) async fn new(body: hyper::Body) -> Data {
        trace_!("Data::new({:?})", body);

        let mut stream = body.compat().map_err(|e| {
            io::Error::new(io::ErrorKind::Other, e)
        }).into_async_read();

        let mut peek_buf = vec![0; PEEK_BYTES];

        let eof = match stream.read_max(&mut peek_buf[..]).await {
            Ok(n) => {
                trace_!("Filled peek buf with {} bytes.", n);

                // TODO.async: This has not gone away, and I don't entirely
                // understand what's happening here

                // We can use `set_len` here instead of `truncate`, but we'll
                // take the performance hit to avoid `unsafe`. All of this code
                // should go away when we migrate away from hyper 0.10.x.

                peek_buf.truncate(n);
                n < PEEK_BYTES
            }
            Err(e) => {
                error_!("Failed to read into peek buffer: {:?}.", e);
                // Likewise here as above.
                peek_buf.truncate(0);
                false
            }
        };

        trace_!("Peek bytes: {}/{} bytes.", peek_buf.len(), PEEK_BYTES);
        Data { buffer: peek_buf, stream: Box::new(stream), is_complete: eof }
    }

    /// This creates a `data` object from a local data source `data`.
    #[inline]
    pub(crate) fn local(data: Vec<u8>) -> Data {
        Data {
            buffer: data,
            stream: Box::new(&[][..]),
            is_complete: true,
        }
    }
}

impl std::borrow::Borrow<()> for Data {
    fn borrow(&self) -> &() {
        &()
    }
}
