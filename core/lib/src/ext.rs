use std::{io, time::Duration};
use std::task::{Poll, Context};
use std::pin::Pin;

use bytes::{Bytes, BytesMut};
use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::time::{sleep, Sleep};

use futures::stream::Stream;
use futures::future::{self, Future, FutureExt};

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
}

pin_project! {
    /// I/O that can be cancelled when a future `F` resolves.
    #[must_use = "futures do nothing unless polled"]
    pub struct CancellableIo<F, I> {
        #[pin]
        io: Option<I>,
        #[pin]
        trigger: future::Fuse<F>,
        state: State,
        grace: Duration,
        mercy: Duration,
    }
}

impl<F: Future, I: AsyncWrite> CancellableIo<F, I> {
    pub fn new(trigger: F, io: I, grace: Duration, mercy: Duration) -> Self {
        CancellableIo {
            grace, mercy,
            io: Some(io),
            trigger: trigger.fuse(),
            state: State::Active,
        }
    }

    pub fn io(&self) -> Option<&I> {
        self.io.as_ref()
    }

    /// Run `do_io` while connection processing should continue.
    fn poll_trigger_then<T>(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        do_io: impl FnOnce(Pin<&mut I>, &mut Context<'_>) -> Poll<io::Result<T>>,
    ) -> Poll<io::Result<T>> {
        let mut me = self.as_mut().project();
        let io = match me.io.as_pin_mut() {
            Some(io) => io,
            None => return Poll::Ready(Err(gone())),
        };

        loop {
            match me.state {
                State::Active => {
                    if me.trigger.as_mut().poll(cx).is_ready() {
                        *me.state = State::Grace(Box::pin(sleep(*me.grace)));
                    } else {
                        return do_io(io, cx);
                    }
                }
                State::Grace(timer) => {
                    if timer.as_mut().poll(cx).is_ready() {
                        *me.state = State::Mercy(Box::pin(sleep(*me.mercy)));
                    } else {
                        return do_io(io, cx);
                    }
                }
                State::Mercy(timer) => {
                    if timer.as_mut().poll(cx).is_ready() {
                        self.project().io.set(None);
                        return Poll::Ready(Err(time_out()));
                    } else {
                        let result = futures::ready!(io.poll_shutdown(cx));
                        self.project().io.set(None);
                        return match result {
                            Err(e) => Poll::Ready(Err(e)),
                            Ok(()) => Poll::Ready(Err(gone()))
                        };
                    }
                },
            }
        }
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
        self.io().map(|io| io.is_write_vectored()).unwrap_or(false)
    }
}

use crate::http::private::{Listener, Connection, Certificates};

impl<F: Future, C: Connection> Connection for CancellableIo<F, C> {
    fn peer_address(&self) -> Option<std::net::SocketAddr> {
        self.io().and_then(|io| io.peer_address())
    }

    fn peer_certificates(&self) -> Option<Certificates> {
        self.io().and_then(|io| io.peer_certificates())
    }

    fn enable_nodelay(&self) -> io::Result<()> {
        match self.io() {
            Some(io) => io.enable_nodelay(),
            None => Ok(())
        }
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

pub trait StreamExt: Sized + Stream {
    fn join<U>(self, other: U) -> Join<Self, U>
        where U: Stream<Item = Self::Item>;
}

impl<S: Stream> StreamExt for S {
    fn join<U>(self, other: U) -> Join<Self, U>
        where U: Stream<Item = Self::Item>
    {
        Join::new(self, other)
    }
}

pin_project! {
    /// Stream returned by the [`join`](super::StreamExt::join) method.
    pub struct Join<T, U> {
        #[pin]
        a: T,
        #[pin]
        b: U,
        // When `true`, poll `a` first, otherwise, `poll` b`.
        toggle: bool,
        // Set when either `a` or `b` return `None`.
        done: bool,
    }
}

impl<T, U> Join<T, U> {
    pub(super) fn new(a: T, b: U) -> Join<T, U>
        where T: Stream, U: Stream,
    {
        Join { a, b, toggle: false, done: false, }
    }

    fn poll_next<A: Stream, B: Stream<Item = A::Item>>(
        first: Pin<&mut A>,
        second: Pin<&mut B>,
        done: &mut bool,
        cx: &mut Context<'_>,
    ) -> Poll<Option<A::Item>> {
        match first.poll_next(cx) {
            Poll::Ready(opt) => { *done = opt.is_none(); Poll::Ready(opt) }
            Poll::Pending => match second.poll_next(cx) {
                Poll::Ready(opt) => { *done = opt.is_none(); Poll::Ready(opt) }
                Poll::Pending => Poll::Pending
            }
        }
    }
}

impl<T, U> Stream for Join<T, U>
    where T: Stream,
          U: Stream<Item = T::Item>,
{
    type Item = T::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T::Item>> {
        if self.done {
            return Poll::Ready(None);
        }

        let me = self.project();
        *me.toggle = !*me.toggle;
        match *me.toggle {
            true => Self::poll_next(me.a, me.b, me.done, cx),
            false => Self::poll_next(me.b, me.a, me.done, cx),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (left_low, left_high) = self.a.size_hint();
        let (right_low, right_high) = self.b.size_hint();

        let low = left_low.saturating_add(right_low);
        let high = match (left_high, right_high) {
            (Some(h1), Some(h2)) => h1.checked_add(h2),
            _ => None,
        };

        (low, high)
    }
}
