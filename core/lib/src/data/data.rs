use std::io::Cursor;

use crate::http::hyper;
use crate::ext::AsyncReadBody;
use crate::tokio::io::AsyncReadExt;
use crate::data::data_stream::DataStream;
use crate::data::ByteUnit;

/// The number of bytes to read into the "peek" buffer.
pub const PEEK_BYTES: usize = 512;

/// Type representing the data in the body of an incoming request.
///
/// This type is the only means by which the body of a request can be retrieved.
/// This type is not usually used directly. Instead, types that implement
/// [`FromTransformedData`](crate::data::FromTransformedData) are used via code
/// generation by specifying the `data = "<var>"` route parameter as follows:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # type DataGuard = rocket::data::Data;
/// #[post("/submit", data = "<var>")]
/// fn submit(var: DataGuard) { /* ... */ }
/// # fn main() { }
/// ```
///
/// Above, `DataGuard` can be any type that implements `FromTransformedData` (or
/// equivalently, `FromData`). Note that `Data` itself implements
/// `FromTransformedData`.
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
    stream: AsyncReadBody,
}

impl Data {
    pub(crate) async fn from_hyp(body: hyper::Body) -> Data {
        // TODO.async: This used to also set the read timeout to 5 seconds.
        // Such a short read timeout is likely no longer necessary, but some
        // kind of idle timeout should be implemented.

        let stream = AsyncReadBody::from(body);
        let buffer = Vec::with_capacity(PEEK_BYTES / 8);
        Data { buffer, stream, is_complete: false }
    }

    /// This creates a `data` object from a local data source `data`.
    #[inline]
    pub(crate) fn local(data: Vec<u8>) -> Data {
        Data {
            buffer: data,
            stream: AsyncReadBody::empty(),
            is_complete: true,
        }
    }

    /// Returns the raw data stream, limited to `limit` bytes.
    ///
    /// The stream contains all of the data in the body of the request,
    /// including that in the `peek` buffer. The method consumes the `Data`
    /// instance. This ensures that a `Data` type _always_ represents _all_ of
    /// the data in a request.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::data::{Data, ToByteUnit};
    ///
    /// # const SIZE_LIMIT: u64 = 2 << 20; // 2MiB
    /// fn handler(data: Data) {
    ///     let stream = data.open(2.mebibytes());
    /// }
    /// ```
    pub fn open(self, limit: ByteUnit) -> DataStream {
        let buffer_limit = std::cmp::min(self.buffer.len().into(), limit);
        let stream_limit = limit - buffer_limit;
        let buffer = Cursor::new(self.buffer).take(buffer_limit.into());
        let stream = self.stream.take(stream_limit.into());
        DataStream { buffer, stream }
    }

    /// Retrieve at most `num` bytes from the `peek` buffer without consuming
    /// `self`.
    ///
    /// The peek buffer contains at most 512 bytes of the body of the request.
    /// The actual size of the returned buffer is the `max` of the request's
    /// body, `num` and `512`. The [`peek_complete`](#method.peek_complete)
    /// method can be used to determine if this buffer contains _all_ of the
    /// data in the body of the request.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::request::Request;
    /// use rocket::data::{self, Data, FromData};
    /// # struct MyType;
    /// # type MyError = String;
    ///
    /// #[rocket::async_trait]
    /// impl FromData for MyType {
    ///     type Error = MyError;
    ///
    ///     async fn from_data(req: &Request<'_>, mut data: Data) -> data::Outcome<Self, MyError> {
    ///         if data.peek(10).await != b"hi" {
    ///             return data::Outcome::Forward(data)
    ///         }
    ///
    ///         /* .. */
    ///         # unimplemented!()
    ///     }
    /// }
    /// ```
    pub async fn peek(&mut self, num: usize) -> &[u8] {
        let num = std::cmp::min(PEEK_BYTES, num);
        let mut len = self.buffer.len();
        if len >= num {
            return &self.buffer[..num];
        }

        while len < num {
            match self.stream.read_buf(&mut self.buffer).await {
                Ok(0) => { self.is_complete = true; break },
                Ok(n) => len += n,
                Err(e) => {
                    error_!("Failed to read into peek buffer: {:?}.", e);
                    break;
                }
            }
        }

        &self.buffer[..std::cmp::min(len, num)]
    }

    /// Returns true if the `peek` buffer contains all of the data in the body
    /// of the request. Returns `false` if it does not or if it is not known if
    /// it does.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::data::Data;
    ///
    /// async fn handler(mut data: Data) {
    ///     if data.peek_complete() {
    ///         println!("All of the data: {:?}", data.peek(512).await);
    ///     }
    /// }
    /// ```
    #[inline(always)]
    pub fn peek_complete(&self) -> bool {
        self.is_complete
    }
}
