use std::pin::Pin;
use std::task::{Context, Poll};

use futures::stream::Stream;

/// A stream that yields exactly one value.
///
/// A `ReaderStream` which wraps this type and yields one `AsyncRead` is
/// returned by [`ReaderStream::one()`]. A `One` can also be constructed via
/// [`One::from()`].
///
/// [`ReaderStream::one()`]: crate::response::stream::ReaderStream::one()
///
/// # Example
///
/// ```rust
/// use rocket::response::stream::One;
/// use rocket::futures::stream::StreamExt;
///
/// # rocket::async_test(async {
/// let mut stream = One::from("hello!");
/// let values: Vec<_> = stream.collect().await;
/// assert_eq!(values, ["hello!"]);
/// # });
/// ```
pub struct One<T: Unpin>(Option<T>);

/// Returns a `One` stream that will yield `value` exactly once.
///
/// # Example
///
/// ```rust
/// use rocket::response::stream::One;
///
/// let mut stream = One::from("hello!");
/// ```
impl<T: Unpin> From<T> for One<T> {
    fn from(value: T) -> Self {
        One(Some(value))
    }
}

impl<T: Unpin> Stream for One<T> {
    type Item = T;

    fn poll_next(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        Poll::Ready(self.0.take())
    }
}
