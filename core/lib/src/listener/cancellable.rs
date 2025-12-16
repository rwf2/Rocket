use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::future::FutureExt;
use futures::Stream;
use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use crate::shutdown::Stages;

pin_project! {
    /// I/O that can be cancelled when a future `F` resolves.
    #[must_use = "futures do nothing unless polled"]
    pub struct Cancellable<I> {
        #[pin]
        io: Option<I>,
        stages: Stages,
        state: State,
    }
}

#[derive(Debug)]
enum State {
    /// I/O has not been cancelled. Proceed as normal until `Shutdown`.
    Active,
    /// I/O has been cancelled. Try to finish before `Shutdown`.
    Grace,
    /// Grace has elapsed. Shutdown connections. After `Shutdown`, force close.
    Mercy,
}

pub trait CancellableExt: Sized {
    fn cancellable(self, stages: Stages) -> Cancellable<Self> {
        Cancellable {
            io: Some(self),
            state: State::Active,
            stages,
        }
    }
}

impl<T> CancellableExt for T {}

fn time_out() -> io::Error {
    io::Error::new(io::ErrorKind::TimedOut, "shutdown grace period elapsed")
}

fn gone() -> io::Error {
    io::Error::new(io::ErrorKind::BrokenPipe, "I/O driver terminated")
}

impl<I: AsyncCancel> Cancellable<I> {
    pub fn inner(&self) -> Option<&I> {
        self.io.as_ref()
    }
}

pub trait AsyncCancel {
    fn poll_cancel(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>>;
}

impl<T: AsyncWrite> AsyncCancel for T {
    fn poll_cancel(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        <T as AsyncWrite>::poll_shutdown(self, cx)
    }
}

impl<I: AsyncCancel> Cancellable<I> {
    /// Run `do_io` while connection processing should continue.
    pub fn poll_with<T>(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        do_io: impl FnOnce(Pin<&mut I>, &mut Context<'_>) -> Poll<io::Result<T>>,
    ) -> Poll<io::Result<T>> {
        let me = self.as_mut().project();
        let io = match me.io.as_pin_mut() {
            Some(io) => io,
            None => return Poll::Ready(Err(gone())),
        };

        loop {
            match me.state {
                State::Active => {
                    if me.stages.start.poll_unpin(cx).is_ready() {
                        *me.state = State::Grace;
                    } else {
                        return do_io(io, cx);
                    }
                }
                State::Grace => {
                    if me.stages.grace.poll_unpin(cx).is_ready() {
                        *me.state = State::Mercy;
                    } else {
                        return do_io(io, cx);
                    }
                }
                State::Mercy => {
                    if me.stages.mercy.poll_unpin(cx).is_ready() {
                        self.project().io.set(None);
                        return Poll::Ready(Err(time_out()));
                    } else {
                        let result = futures::ready!(io.poll_cancel(cx));
                        self.project().io.set(None);
                        return match result {
                            Ok(()) => Poll::Ready(Err(gone())),
                            Err(e) => Poll::Ready(Err(e)),
                        };
                    }
                }
            }
        }
    }
}

impl<I: AsyncRead + AsyncCancel> AsyncRead for Cancellable<I> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        self.poll_with(cx, |io, cx| io.poll_read(cx, buf))
    }
}

impl<I: AsyncWrite> AsyncWrite for Cancellable<I> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.poll_with(cx, |io, cx| io.poll_write(cx, buf))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.poll_with(cx, |io, cx| io.poll_flush(cx))
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.poll_with(cx, |io, cx| io.poll_shutdown(cx))
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        self.poll_with(cx, |io, cx| io.poll_write_vectored(cx, bufs))
    }

    fn is_write_vectored(&self) -> bool {
        self.inner()
            .map(|io| io.is_write_vectored())
            .unwrap_or(false)
    }
}

impl<T, I: Stream<Item = io::Result<T>> + AsyncCancel> Stream for Cancellable<I> {
    type Item = I::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        use futures::ready;

        match ready!(self.poll_with(cx, |io, cx| io.poll_next(cx).map(Ok))) {
            Ok(Some(v)) => Poll::Ready(Some(v)),
            Ok(None) => Poll::Ready(None),
            Err(e) => Poll::Ready(Some(Err(e))),
        }
    }
}
