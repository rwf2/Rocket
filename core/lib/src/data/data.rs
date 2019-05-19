use std::io::{self, Read, Write, Cursor, Chain};
use std::path::Path;
use std::fs::File;
use std::time::Duration;

#[cfg(feature = "tls")] use super::net_stream::HttpsStream;

use super::data_stream::{DataStream, /* TODO kill_stream */};
use super::net_stream::NetStream;
use crate::ext::ReadExt;

use crate::http::hyper::{self, Payload};
use futures::{Async, Future};

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
/// # #![feature(proc_macro_hygiene)]
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
    body: Vec<u8>,
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
        // FIXME: Insert a `BufReader` in front of the `NetStream` with capacity
        // 4096. We need the new `Chain` methods to get the inner reader to
        // actually do this, however.
        let stream = ::std::mem::replace(&mut self.body, vec![]);
        DataStream(Cursor::new(stream))
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
        if self.body.len() > PEEK_BYTES {
            &self.body[..PEEK_BYTES]
        } else {
            &self.body
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
        // TODO self.is_complete
        true
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
    crate fn new(body: Vec<u8>) -> Data {
        Data { body }
    }

}
