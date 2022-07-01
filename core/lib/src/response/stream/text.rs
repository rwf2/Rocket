use futures::stream::{Stream, StreamExt};

use crate::request::Request;
use crate::response::{self, Response, Responder};
use crate::http::ContentType;
use crate::response::stream::ReaderStream;

/// A potentially infinite stream of text: `T: AsRef<str>`.
///
/// A `TextStream` can be constructed from any [`Stream`] of items of type `T`
/// where `T: AsRef<str>`. This includes `&str`, `String`, `Cow<str>`,
/// `&RawStr`, and more. The stream can be constructed directly, via
/// `TextStream(..)` or [`TextStream::from()`], or through generator syntax via
/// [`TextStream!`].
///
/// [`Stream`]: https://docs.rs/futures/0.3/futures/stream/trait.Stream.html
///
/// # Responder
///
/// `TextStream` is a (potentially infinite) responder. The response
/// `Content-Type` is set to [`Text`](ContentType::Text). The body is
/// [unsized](crate::response::Body#unsized), and values are sent as soon as
/// they are yielded by the internal iterator.
///
/// # Example
///
/// Use [`TextStream!`] to yield 10 strings, one every second, of the form `"n:
/// $k"` for `$k` from `0` to `10` exclusive:
///
/// ```rust
/// # use rocket::*;
/// use rocket::response::stream::TextStream;
/// use rocket::futures::stream::{repeat, StreamExt};
/// use rocket::tokio::time::{self, Duration};
///
/// #[get("/text")]
/// fn text() -> TextStream![&'static str] {
///     TextStream(repeat("hi"))
/// }
///
/// #[get("/text/stream")]
/// fn stream() -> TextStream![String] {
///     TextStream! {
///         let mut interval = time::interval(Duration::from_secs(1));
///         for i in 0..10 {
///             yield format!("n: {}", i);
///             interval.tick().await;
///         }
///     }
/// }
/// ```
///
/// The syntax of [`TextStream!`] as an expression is identical to that of
/// [`stream!`](crate::response::stream::stream).
#[derive(Debug, Clone)]
pub struct TextStream<S>(pub S);

impl<S> From<S> for TextStream<S> {
    /// Creates a `TextStream` from any `S: Stream`.
    fn from(stream: S) -> Self {
        TextStream(stream)
    }
}

impl<'r, S: Stream> Responder<'r, 'r> for TextStream<S>
    where S: Send + 'r, S::Item: AsRef<str> + Send + Unpin + 'r
{
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'r> {
        struct ByteStr<T>(T);

        impl<T: AsRef<str>> AsRef<[u8]> for ByteStr<T> {
            fn as_ref(&self) -> &[u8] {
                self.0.as_ref().as_bytes()
            }
        }

        let inner = self.0.map(ByteStr).map(std::io::Cursor::new);
        Response::build()
            .header(ContentType::Text)
            .streamed_body(ReaderStream::from(inner))
            .ok()
    }
}

crate::export! {
    /// Type and stream expression macro for [`struct@TextStream`].
    ///
    /// See [`stream!`](crate::response::stream::stream) for the syntax
    /// supported by this macro.
    ///
    /// See [`struct@TextStream`] and the [module level
    /// docs](crate::response::stream#typed-streams) for usage details.
    macro_rules! TextStream {
        ($($s:tt)*) => ($crate::_typed_stream!(TextStream, $($s)*));
    }
}
