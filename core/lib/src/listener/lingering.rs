use std::io;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;

use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::time::{sleep, Sleep};

pin_project! {
    /// I/O that lingers when shut down, unless the connection was upgraded.
    ///
    /// Closing a connection with unread data sends a TCP reset, which can
    /// discard a response the remote has not yet read (RFC 9112 §9.6). Instead,
    /// after shutting down writes, read and discard data until the remote
    /// closes the connection, reading fails, or `linger` elapses. When `I` is a
    /// `Cancellable`, cancellation still ends the connection at the end of the
    /// grace period, as reads fail from then on.
    ///
    /// An upgraded connection belongs to its I/O handler, which may shut down
    /// writes and continue to read. As such, once `upgraded` is set, shutting
    /// down only shuts down writes, and incoming data is left unread.
    #[must_use = "futures do nothing unless polled"]
    pub struct Lingering<I> {
        #[pin]
        io: I,
        linger: Duration,
        upgraded: Arc<AtomicBool>,
        lingering: Option<Pin<Box<Sleep>>>,
    }
}

impl<I> Lingering<I> {
    pub fn new(io: I, linger: Duration, upgraded: Arc<AtomicBool>) -> Self {
        Lingering { io, linger, upgraded, lingering: None }
    }
}

impl<I: AsyncRead> AsyncRead for Lingering<I> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        self.project().io.poll_read(cx, buf)
    }
}

impl<I: AsyncRead + AsyncWrite> AsyncWrite for Lingering<I> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.project().io.poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // Writes were shut down, so there is nothing left to flush. Hyper
        // flushes before every attempt to shut down, and once a lingering
        // connection is cancelled, flushing it would fail.
        if self.lingering.is_some() {
            return Poll::Ready(Ok(()));
        }

        self.project().io.poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>
    ) -> Poll<io::Result<()>> {
        if self.lingering.is_none() {
            let me = self.as_mut().project();
            futures::ready!(me.io.poll_shutdown(cx))?;
            if me.linger.is_zero() || me.upgraded.load(Ordering::Acquire) {
                return Poll::Ready(Ok(()));
            }

            *me.lingering = Some(Box::pin(sleep(*me.linger)));
        }

        let mut buf = [0; 4096];
        loop {
            let me = self.as_mut().project();
            if me.lingering.as_mut().map_or(true, |timer| timer.as_mut().poll(cx).is_ready()) {
                return Poll::Ready(Ok(()));
            }

            let mut buf = ReadBuf::new(&mut buf);
            match futures::ready!(me.io.poll_read(cx, &mut buf)) {
                Ok(()) if !buf.filled().is_empty() => continue,
                _ => return Poll::Ready(Ok(())),
            }
        }
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        self.project().io.poll_write_vectored(cx, bufs)
    }

    fn is_write_vectored(&self) -> bool {
        self.io.is_write_vectored()
    }
}
