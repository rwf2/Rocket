use std::{io, time::Duration};
use std::task::{Poll, Context};
use std::pin::Pin;

use bytes::BytesMut;
use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::time::{sleep, Sleep};

use futures::stream::Stream;
use futures::future::{Future, Fuse, FutureExt};

use crate::http::hyper::Bytes;

pin_project! {
    pub struct ReaderStream<R> {
        #[pin]
        reader: Option<R>,
        buf: BytesMut,
        cap: usize,
    }
}

impl<R: AsyncRead> Stream for ReaderStream<R> {
    type Item = std::io::Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        use tokio_util::io::poll_read_buf;

        let mut this = self.as_mut().project();

        let reader = match this.reader.as_pin_mut() {
            Some(r) => r,
            None => return Poll::Ready(None),
        };

        if this.buf.capacity() == 0 {
            this.buf.reserve(*this.cap);
        }

        match poll_read_buf(reader, cx, &mut this.buf) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(err)) => {
                self.project().reader.set(None);
                Poll::Ready(Some(Err(err)))
            }
            Poll::Ready(Ok(0)) => {
                self.project().reader.set(None);
                Poll::Ready(None)
            }
            Poll::Ready(Ok(_)) => {
                let chunk = this.buf.split();
                Poll::Ready(Some(Ok(chunk.freeze())))
            }
        }
    }
}

pub trait AsyncReadExt: AsyncRead + Sized {
    fn into_bytes_stream(self, cap: usize) -> ReaderStream<Self> {
        ReaderStream { reader: Some(self), cap, buf: BytesMut::with_capacity(cap) }
    }
}

impl<T: AsyncRead> AsyncReadExt for T { }

pub trait PollExt<T, E> {
    fn map_err_ext<U, F>(self, f: F) -> Poll<Option<Result<T, U>>>
        where F: FnOnce(E) -> U;
}

impl<T, E> PollExt<T, E> for Poll<Option<Result<T, E>>> {
    /// Changes the error value of this `Poll` with the closure provided.
    fn map_err_ext<U, F>(self, f: F) -> Poll<Option<Result<T, U>>>
        where F: FnOnce(E) -> U
    {
        match self {
            Poll::Ready(Some(Ok(t))) => Poll::Ready(Some(Ok(t))),
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(f(e)))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

pin_project! {
    /// Stream for the [`chain`](super::AsyncReadExt::chain) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct Chain<T, U> {
        #[pin]
        first: T,
        #[pin]
        second: U,
        done_first: bool,
    }
}

impl<T: AsyncRead, U: AsyncRead> Chain<T, U> {
    pub(crate) fn new(first: T, second: U) -> Self {
        Self { first, second, done_first: false }
    }
}

impl<T: AsyncRead, U: AsyncRead> Chain<T, U> {
    /// Gets references to the underlying readers in this `Chain`.
    pub fn get_ref(&self) -> (&T, &U) {
        (&self.first, &self.second)
    }
}

impl<T: AsyncRead, U: AsyncRead> AsyncRead for Chain<T, U> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let me = self.project();

        if !*me.done_first {
            let init_rem = buf.remaining();
            futures::ready!(me.first.poll_read(cx, buf))?;
            if buf.remaining() == init_rem {
                *me.done_first = true;
            } else {
                return Poll::Ready(Ok(()));
            }
        }
        me.second.poll_read(cx, buf)
    }
}

enum State {
    /// I/O has not been cancelled. Proceed as normal.
    Active,
    /// I/O has been cancelled. See if we can finish before the timer expires.
    Grace(Pin<Box<Sleep>>),
    /// Grace period elapsed. Shutdown the connection, waiting for the timer
    /// until we force close.
    Mercy(Pin<Box<Sleep>>),
    /// We failed to shutdown and are force-closing the connection.
    Terminated,
    /// We successfuly shutdown the connection.
    Inactive,
}

pin_project! {
    /// I/O that can be cancelled when a future `F` resolves.
    #[must_use = "futures do nothing unless polled"]
    pub struct CancellableIo<F, I> {
        #[pin]
        io: I,
        #[pin]
        trigger: Fuse<F>,
        state: State,
        grace: Duration,
        mercy: Duration,
    }
}

impl<F: Future, I: AsyncWrite> CancellableIo<F, I> {
    pub fn new(trigger: F, io: I, grace: Duration, mercy: Duration) -> Self {
        CancellableIo {
            io, grace, mercy,
            trigger: trigger.fuse(),
            state: State::Active
        }
    }

    /// Returns `Ok(true)` if connection processing should continue.
    fn poll_trigger_then<T>(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        io: impl FnOnce(Pin<&mut I>, &mut Context<'_>) -> Poll<io::Result<T>>,
    ) -> Poll<io::Result<T>> {
        let mut me = self.project();

        // CORRECTNESS: _EVERY_ branch must reset `state`! If `state` is
        // unchanged in a branch, that branch _must_ `break`! No `return`!
        let mut state = std::mem::replace(me.state, State::Active);
        let result = loop {
            match state {
                State::Active => {
                    if me.trigger.as_mut().poll(cx).is_ready() {
                        state = State::Grace(Box::pin(sleep(*me.grace)));
                    } else {
                        state = State::Active;
                        break io(me.io, cx);
                    }
                }
                State::Grace(mut sleep) => {
                    if sleep.as_mut().poll(cx).is_ready() {
                        if let Some(deadline) = sleep.deadline().checked_add(*me.mercy) {
                            sleep.as_mut().reset(deadline);
                            state = State::Mercy(sleep);
                        } else {
                            state = State::Terminated;
                        }
                    } else {
                        state = State::Grace(sleep);
                        break io(me.io, cx);
                    }
                },
                State::Mercy(mut sleep) => {
                    if sleep.as_mut().poll(cx).is_ready() {
                        state = State::Terminated;
                        continue;
                    }

                    match me.io.as_mut().poll_shutdown(cx) {
                        Poll::Ready(Err(e)) => {
                            state = State::Terminated;
                            break Poll::Ready(Err(e));
                        }
                        Poll::Ready(Ok(())) => {
                            state = State::Inactive;
                            break Poll::Ready(Err(gone()));
                        }
                        Poll::Pending => {
                            state = State::Mercy(sleep);
                            break Poll::Pending;
                        }
                    }
                },
                State::Terminated => {
                    // Just in case, as a last ditch effort. Ignore pending.
                    state = State::Terminated;
                    let _ = me.io.as_mut().poll_shutdown(cx);
                    break Poll::Ready(Err(time_out()));
                },
                State::Inactive => {
                    state = State::Inactive;
                    break Poll::Ready(Err(gone()));
                }
            }
        };

        *me.state = state;
        result
    }
}

fn time_out() -> io::Error {
    io::Error::new(io::ErrorKind::TimedOut, "Shutdown grace timed out")
}

fn gone() -> io::Error {
    io::Error::new(io::ErrorKind::BrokenPipe, "IO driver has terminated")
}

impl<F: Future, I: AsyncRead + AsyncWrite> AsyncRead for CancellableIo<F, I> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        self.as_mut().poll_trigger_then(cx, |io, cx| io.poll_read(cx, buf))
    }
}

impl<F: Future, I: AsyncWrite> AsyncWrite for CancellableIo<F, I> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.as_mut().poll_trigger_then(cx, |io, cx| io.poll_write(cx, buf))
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>
    ) -> Poll<io::Result<()>> {
        self.as_mut().poll_trigger_then(cx, |io, cx| io.poll_flush(cx))
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>
    ) -> Poll<io::Result<()>> {
        self.as_mut().poll_trigger_then(cx, |io, cx| io.poll_shutdown(cx))
    }

    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        self.as_mut().poll_trigger_then(cx, |io, cx| io.poll_write_vectored(cx, bufs))
    }

    fn is_write_vectored(&self) -> bool {
        self.io.is_write_vectored()
    }
}

use crate::http::private::{Listener, Connection};

impl<F: Future, C: Connection> Connection for CancellableIo<F, C> {
    fn remote_addr(&self) -> Option<std::net::SocketAddr> {
        self.io.remote_addr()
    }
}

pin_project! {
    pub struct CancellableListener<F, L> {
        pub trigger: F,
        #[pin]
        pub listener: L,
        pub grace: Duration,
        pub mercy: Duration,
    }
}

impl<F, L> CancellableListener<F, L> {
    pub fn new(trigger: F, listener: L, grace: u64, mercy: u64) -> Self {
        let (grace, mercy) = (Duration::from_secs(grace), Duration::from_secs(mercy));
        CancellableListener { trigger, listener, grace, mercy }
    }
}

impl<L: Listener, F: Future + Clone> Listener for CancellableListener<F, L> {
    type Connection = CancellableIo<F, L::Connection>;

    fn local_addr(&self) -> Option<std::net::SocketAddr> {
        self.listener.local_addr()
    }

    fn poll_accept(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>
    ) -> Poll<io::Result<Self::Connection>> {
        self.as_mut().project().listener
            .poll_accept(cx)
            .map(|res| res.map(|conn| {
                CancellableIo::new(self.trigger.clone(), conn, self.grace, self.mercy)
            }))
    }
}
