mod chain;
mod reader_stream;
mod join;
mod waker;

#[cfg(unix)]
pub mod unix;

pub use chain::Chain;
pub use reader_stream::ReaderStream;
pub use join::join;
pub use waker::RuntimeWaker;

#[track_caller]
pub fn spawn_inspect<E, F, Fut>(or: F, future: Fut)
    where F: FnOnce(&E) + Send + Sync + 'static,
          E: Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(), E>> + Send + 'static,
{
    use futures::TryFutureExt;
    tokio::spawn(future.inspect_err(or));
}

use std::{fmt, io};
use std::pin::{pin, Pin};
use std::future::Future;
use std::task::{ready, Context, Poll};
use either::Either;
use futures::future;
use tokio::runtime::Handle;

pub trait FutureExt: Future + Sized {
    /// Await `self` or `other`, whichever finishes first.
    async fn race<B: Future>(self, other: B) -> Either<Self::Output, B::Output> {
        match future::select(pin!(self), pin!(other)).await {
            future::Either::Left((v, _)) => Either::Left(v),
            future::Either::Right((v, _)) => Either::Right(v),
        }
    }

    async fn race_io<T, K: Future>(self, trigger: K) -> io::Result<T>
        where Self: Future<Output = io::Result<T>>
    {
        match future::select(pin!(self), pin!(trigger)).await {
            future::Either::Left((v, _)) => v,
            future::Either::Right((_, _)) => Err(io::Error::other("i/o terminated")),
        }
    }
}

impl<F: Future + Sized> FutureExt for F { }

/// A future that, if dropped before it completes, is spawned to run to
/// completion on the current runtime.
pub struct Detach<F: Future + Send + 'static>(Option<Pin<Box<F>>>);

impl<F: Future + Send + 'static> Detach<F> {
    pub fn new(future: F) -> Self {
        Detach(Some(Box::pin(future)))
    }
}

impl<F: Future + Send + 'static> Future for Detach<F> {
    type Output = F::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let future = self.0.as_mut().expect("`Detach` polled after completion");
        let output = ready!(future.as_mut().poll(cx));
        self.0 = None;
        Poll::Ready(output)
    }
}

impl<F: Future + Send + 'static> Drop for Detach<F> {
    fn drop(&mut self) {
        if let (Some(future), Ok(runtime)) = (self.0.take(), Handle::try_current()) {
            runtime.spawn(async move { future.await; });
        }
    }
}

pub struct Formatter<F: Fn(&mut fmt::Formatter<'_>) -> fmt::Result>(pub F);

impl<F: Fn(&mut fmt::Formatter<'_>) -> fmt::Result> fmt::Debug for Formatter<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.0)(f)
    }
}

impl<F: Fn(&mut fmt::Formatter<'_>) -> fmt::Result> fmt::Display for Formatter<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.0)(f)
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! for_both {
    ($value:expr, $pattern:pat => $result:expr) => {
        match $value {
            tokio_util::either::Either::Left($pattern) => $result,
            tokio_util::either::Either::Right($pattern) => $result,
        }
    };
}

pub use for_both;
