use std::pin::Pin;
use std::task::{Context, Poll};

use pin_project_lite::pin_project;

use futures::ready;
use futures::stream::Stream;

/// Join two streams, `a` and `b`, into a new `Join` stream that returns items
/// from both streams, biased to `a`, until `a` finishes. The joined stream
/// completes when `a` completes, irrespective of `b`. If `b` stops producing
/// values, then the joined stream acts exactly like a fused `a`.
///
/// Values are biased to those of `a`: if `a` provides a value, it is always
/// emitted before a value provided by `b`. In other words, values from `b` are
/// emitted when and only when `a` is not producing a value.
pub fn join<A: Stream, B: Stream>(a: A, b: B) -> Join<A, B> {
    Join {
        a,
        b: Some(b),
        done: false,
    }
}

pin_project! {
    /// Stream returned by [`join`].
    pub struct Join<T, U> {
        #[pin]
        a: T,
        #[pin]
        b: Option<U>,
        // Set when `a` returns `None`.
        done: bool,
    }
}

impl<T, U> Stream for Join<T, U>
where
    T: Stream,
    U: Stream<Item = T::Item>,
{
    type Item = T::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T::Item>> {
        if self.done {
            return Poll::Ready(None);
        }

        let me = self.as_mut().project();
        match me.a.poll_next(cx) {
            Poll::Ready(opt) => {
                *me.done = opt.is_none();
                Poll::Ready(opt)
            }
            Poll::Pending => match me.b.as_pin_mut() {
                None => Poll::Pending,
                Some(b) => match ready!(b.poll_next(cx)) {
                    Some(value) => Poll::Ready(Some(value)),
                    None => {
                        self.as_mut().project().b.set(None);
                        Poll::Pending
                    }
                },
            },
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (left_low, left_high) = self.a.size_hint();
        let (right_low, right_high) = self.b.as_ref().map(|b| b.size_hint()).unwrap_or_default();

        let low = left_low.saturating_add(right_low);
        let high = match (left_high, right_high) {
            (Some(h1), Some(h2)) => h1.checked_add(h2),
            _ => None,
        };

        (low, high)
    }
}
