use std::io::{self, Read, Write, Cursor, Chain};
use std::path::Path;
use std::fs::File;
use std::time::Duration;

#[cfg(feature = "tls")] use super::net_stream::HttpsStream;

use super::data_stream::{DataStream, kill_stream};
use super::net_stream::NetStream;
use crate::ext::ReadExt;

use crate::http::hyper;
use crate::http::hyper::h1::HttpReader;
use crate::http::hyper::h1::HttpReader::*;
use crate::http::hyper::net::{HttpStream, NetworkStream};

pub type HyperBodyReader<'a, 'b> =
    self::HttpReader<&'a mut hyper::buffer::BufReader<&'b mut dyn NetworkStream>>;

//                              |---- from hyper ----|
pub type BodyReader = HttpReader<Chain<Cursor<Vec<u8>>, NetStream>>;

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
/// # #![feature(proc_macro_hygiene, decl_macro)]
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
    stream: BodyReader,
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
        let empty_stream = Cursor::new(vec![]).chain(NetStream::Empty);

        // FIXME: Insert a `BufReader` in front of the `NetStream` with capacity
        // 4096. We need the new `Chain` methods to get the inner reader to
        // actually do this, however.
        let empty_http_stream = HttpReader::SizedReader(empty_stream, 0);
        let stream = std::mem::replace(&mut self.stream, empty_http_stream);
        DataStream(Cursor::new(buffer).chain(stream))
    }

    // FIXME: This is absolutely terrible (downcasting!), thanks to Hyper.
    crate fn from_hyp(mut body: HyperBodyReader<'_, '_>) -> Result<Data, &'static str> {
        #[inline(always)]
        #[cfg(feature = "tls")]
        fn concrete_stream(stream: &mut dyn NetworkStream) -> Option<NetStream> {
            stream.downcast_ref::<HttpsStream>()
                .map(|s| NetStream::Https(s.clone()))
                .or_else(|| {
                    stream.downcast_ref::<HttpStream>()
                        .map(|s| NetStream::Http(s.clone()))
                })
        }

        #[inline(always)]
        #[cfg(not(feature = "tls"))]
        fn concrete_stream(stream: &mut dyn NetworkStream) -> Option<NetStream> {
            stream.downcast_ref::<HttpStream>()
                .map(|s| NetStream::Http(s.clone()))
        }

        // Retrieve the underlying Http(s)Stream from Hyper.
        let net_stream = match concrete_stream(*body.get_mut().get_mut()) {
            Some(net_stream) => net_stream,
            None => return Err("Stream is not an HTTP(s) stream!")
        };

        // Set the read timeout to 5 seconds.
        let _ = net_stream.set_read_timeout(Some(Duration::from_secs(5)));

        // Steal the internal, undecoded data buffer from Hyper.
        let (mut hyper_buf, pos, cap) = body.get_mut().take_buf();
        hyper_buf.truncate(cap); // slow, but safe
        let mut cursor = Cursor::new(hyper_buf);
        cursor.set_position(pos as u64);

        // Create an HTTP reader from the buffer + stream.
        let inner_data = cursor.chain(net_stream);
        let http_stream = match body {
            SizedReader(_, n) => SizedReader(inner_data, n),
            EofReader(_) => EofReader(inner_data),
            EmptyReader(_) => EmptyReader(inner_data),
            ChunkedReader(_, n) => ChunkedReader(inner_data, n)
        };

        Ok(Data::new(http_stream))
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
    pub fn stream_to<W: Write>(self, writer: &mut W) -> io::Result<u64> {
        io::copy(&mut self.open(), writer)
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
    pub fn stream_to_file<P: AsRef<Path>>(self, path: P) -> io::Result<u64> {
        io::copy(&mut self.open(), &mut File::create(path)?)
    }

    // Creates a new data object with an internal buffer `buf`, where the cursor
    // in the buffer is at `pos` and the buffer has `cap` valid bytes. Thus, the
    // bytes `vec[pos..cap]` are buffered and unread. The remainder of the data
    // bytes can be read from `stream`.
    #[inline(always)]
    crate fn new(mut stream: BodyReader) -> Data {
        trace_!("Data::new({:?})", stream);
        let mut peek_buf: Vec<u8> = vec![0; PEEK_BYTES];

        // Fill the buffer with as many bytes as possible. If we read less than
        // that buffer's length, we know we reached the EOF. Otherwise, it's
        // unclear, so we just say we didn't reach EOF.
        let eof = match stream.read_max(&mut peek_buf[..]) {
            Ok(n) => {
                trace_!("Filled peek buf with {} bytes.", n);
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
            },
        };

        trace_!("Peek bytes: {}/{} bytes.", peek_buf.len(), PEEK_BYTES);
        Data { buffer: peek_buf, stream, is_complete: eof }
    }

    /// This creates a `data` object from a local data source `data`.
    #[inline]
    crate fn local(data: Vec<u8>) -> Data {
        let empty_stream = Cursor::new(vec![]).chain(NetStream::Empty);

        Data {
            buffer: data,
            stream: HttpReader::SizedReader(empty_stream, 0),
            is_complete: true,
        }
    }
}

impl Drop for Data {
    fn drop(&mut self) {
        kill_stream(&mut self.stream);
    }
}
