use std::fmt::{self, Debug};

use tokio::io::AsyncRead;

use crate::request::Request;
use crate::response::{self, Response, Responder, DEFAULT_CHUNK_SIZE};

/// Streams a response to a client from an arbitrary `AsyncRead`er type.
///
/// The client is sent a "chunked" response, where the chunk size is at most
/// 4KiB. This means that at most 4KiB are stored in memory while the response
/// is being sent. This type should be used when sending responses that are
/// arbitrarily large in size, such as when streaming from a local socket.
pub struct Stream<T: AsyncRead>(T, usize);

impl<T: AsyncRead> Stream<T> {
    /// Create a new stream from the given `reader` and sets the chunk size for
    /// each streamed chunk to `chunk_size` bytes.
    ///
    /// # Example
    ///
    /// Stream a response from whatever is in `stdin` with a chunk size of 10
    /// bytes. Note: you probably shouldn't do this.
    ///
    /// ```rust
    /// use rocket::response::Stream;
    ///
    /// # #[allow(unused_variables)]
    /// let response = Stream::chunked(tokio::io::stdin(), 10);
    /// ```
    pub fn chunked(reader: T, chunk_size: usize) -> Stream<T> {
        Stream(reader, chunk_size)
    }
}

impl<T: AsyncRead + Debug> Debug for Stream<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Stream").field(&self.0).finish()
    }
}

/// Create a new stream from the given `reader`.
///
/// # Example
///
/// Stream a response from whatever is in `stdin`. Note: you probably
/// shouldn't do this.
///
/// ```rust
/// use rocket::response::Stream;
///
/// # #[allow(unused_variables)]
/// let response = Stream::from(tokio::io::stdin());
/// ```
impl<T: AsyncRead> From<T> for Stream<T> {
    fn from(reader: T) -> Self {
        Stream(reader, DEFAULT_CHUNK_SIZE)
    }
}

/// Sends a response to the client using the "Chunked" transfer encoding. The
/// maximum chunk size is 4KiB.
///
/// # Failure
///
/// If reading from the input stream fails at any point during the response, the
/// response is abandoned, and the response ends abruptly. An error is printed
/// to the console with an indication of what went wrong.
impl<'r, 'o: 'r, T: AsyncRead + Send + 'o> Responder<'r, 'o> for Stream<T> {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'o> {
        Response::build().chunked_body(self.0, self.1).ok()
    }
}
