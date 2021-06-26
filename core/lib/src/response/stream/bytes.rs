use futures::stream::{Stream, StreamExt};

use crate::request::Request;
use crate::response::{self, Response, Responder};
use crate::http::ContentType;
use crate::response::stream::ReaderStream;

/// A potentially infinite stream of bytes: any `T: AsRef<[u8]>`.
///
/// A `ByteStream` can be constructed from any [`Stream`] of items of type `T`
/// where `T: AsRef<[u8]>`. This includes `Vec<u8>`, `&[u8]`, `&str`, `&RawStr`,
/// and more. The stream can be constructed directly, via `ByteStream(..)` or
/// [`ByteStream::from()`], or through generator syntax via [`ByteStream!`].
///
/// [`Stream`]: https://docs.rs/futures/0.3/futures/stream/trait.Stream.html
///
/// # Responder
///
/// `ByteStream` is a (potentially infinite) responder. The response
/// `Content-Type` is set to [`Binary`](ContentType::Binary). The body is
/// [unsized](crate::response::Body#unsized), and values are sent as soon as
/// they are yielded by the internal iterator.
///
/// # Example
///
/// Use [`ByteStream!`] to yield 10 3-byte vectors, one every second, of the
/// form `vec![i, i + 1, i + 2]` for `i` from `0` to `10` exclusive:
///
/// ```rust
/// # use rocket::*;
/// use rocket::response::stream::ByteStream;
/// use rocket::futures::stream::{repeat, StreamExt};
/// use rocket::tokio::time::{self, Duration};
///
/// #[get("/bytes")]
/// fn bytes() -> ByteStream![&'static [u8]] {
///     ByteStream(repeat(&[1, 2, 3][..]))
/// }
///
/// #[get("/byte/stream")]
/// fn stream() -> ByteStream![Vec<u8>] {
///     ByteStream! {
///         let mut interval = time::interval(Duration::from_secs(1));
///         for i in 0..10u8 {
///             yield vec![i, i + 1, i + 2];
///             interval.tick().await;
///         }
///     }
/// }
/// ```
///
/// The syntax of `ByteStream!` as an expression is identical to that of
/// [`stream!`](crate::response::stream::stream).
#[derive(Debug, Clone)]
pub struct ByteStream<S>(pub S);

impl<S> From<S> for ByteStream<S> {
    /// Creates a `ByteStream` from any `S: Stream`.
    fn from(stream: S) -> Self {
        ByteStream(stream)
    }
}

impl<'r, S: Stream> Responder<'r, 'r> for ByteStream<S>
    where S: Send + 'r, S::Item: AsRef<[u8]> + Send + Unpin + 'r
{
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'r> {
        Response::build()
            .header(ContentType::Binary)
            .streamed_body(ReaderStream::from(self.0.map(std::io::Cursor::new)))
            .ok()
    }
}

crate::export! {
    /// Type and stream expression macro for [`struct@ByteStream`].
    ///
    /// See [`stream!`](crate::response::stream::stream) for the syntax
    /// supported by this macro.
    ///
    /// See [`struct@ByteStream`] and the [module level
    /// docs](crate::response::stream#typed-streams) for usage details.
    macro_rules! ByteStream {
        ($($s:tt)*) => ($crate::_typed_stream!(ByteStream, $($s)*));
    }
}
