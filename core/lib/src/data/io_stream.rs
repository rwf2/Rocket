use std::io;
use std::task::{Context, Poll};
use std::pin::Pin;

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use crate::http::hyper::upgrade::Upgraded;

/// A bidirectional, raw stream to the client.
///
/// An instance of `IoStream` is passed to an [`IoHandler`] in response to a
/// successful upgrade request initiated by responders via
/// [`Response::add_upgrade()`] or the equivalent builder method
/// [`Builder::upgrade()`]. For details on upgrade connections, see
/// [`Response`#upgrading].
///
/// An `IoStream` is guaranteed to be [`AsyncRead`], [`AsyncWrite`], and
/// `Unpin`. Bytes written to the stream are sent directly to the client. Bytes
/// read from the stream are those sent directly _by_ the client. See
/// [`IoHandler`] for one example of how values of this type are used.
///
/// [`Response::add_upgrade()`]: crate::Response::add_upgrade()
/// [`Builder::upgrade()`]: crate::response::Builder::upgrade()
/// [`Response`#upgrading]: crate::response::Response#upgrading
pub struct IoStream {
    kind: IoStreamKind,
}

/// Just in case we want to add stream kinds in the future.
enum IoStreamKind {
    Upgraded(Upgraded)
}

/// An upgraded connection I/O handler.
///
/// An I/O handler performs raw I/O via the passed in [`IoStream`], which is
/// [`AsyncRead`], [`AsyncWrite`], and `Unpin`.
///
/// # Example
///
/// The example below implements an `EchoHandler` that echos the raw bytes back
/// to the client.
///
/// ```rust
/// use std::pin::Pin;
///
/// use rocket::tokio::io;
/// use rocket::data::{IoHandler, IoStream};
///
/// struct EchoHandler;
///
/// #[rocket::async_trait]
/// impl IoHandler for EchoHandler {
///     async fn io(self: Pin<Box<Self>>, io: IoStream) -> io::Result<()> {
///         let (mut reader, mut writer) = io::split(io);
///         io::copy(&mut reader, &mut writer).await?;
///         Ok(())
///     }
/// }
///
/// # use rocket::Response;
/// # rocket::async_test(async {
/// # let mut response = Response::new();
/// # response.add_upgrade("raw-echo", EchoHandler);
/// # assert!(response.upgrade("raw-echo").is_some());
/// # })
/// ```
#[crate::async_trait]
pub trait IoHandler: Send {
    /// Performs the raw I/O.
    async fn io(self: Pin<Box<Self>>, io: IoStream) -> io::Result<()>;
}

#[doc(hidden)]
impl From<Upgraded> for IoStream {
    fn from(io: Upgraded) -> Self {
        IoStream { kind: IoStreamKind::Upgraded(io) }
    }
}

/// A "trait alias" of sorts so we can use `AsyncRead + AsyncWrite + Unpin` in `dyn`.
pub trait AsyncReadWrite: AsyncRead + AsyncWrite + Unpin { }

/// Implemented for all `AsyncRead + AsyncWrite + Unpin`, of course.
impl<T: AsyncRead + AsyncWrite + Unpin> AsyncReadWrite for T {  }

impl IoStream {
    /// Returns the internal I/O stream.
    fn inner_mut(&mut self) -> Pin<&mut dyn AsyncReadWrite> {
        match self.kind {
            IoStreamKind::Upgraded(ref mut io) => Pin::new(io),
        }
    }

    /// Returns `true` if the inner I/O stream is write vectored.
    fn inner_is_write_vectored(&self) -> bool {
        match self.kind {
            IoStreamKind::Upgraded(ref io) => io.is_write_vectored(),
        }
    }
}

impl AsyncRead for IoStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        self.get_mut().inner_mut().poll_read(cx, buf)
    }
}

impl AsyncWrite for IoStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.get_mut().inner_mut().poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.get_mut().inner_mut().poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.get_mut().inner_mut().poll_shutdown(cx)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        self.get_mut().inner_mut().poll_write_vectored(cx, bufs)
    }

    fn is_write_vectored(&self) -> bool {
        self.inner_is_write_vectored()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_unpin() {
        fn check_traits<T: AsyncRead + AsyncWrite + Unpin + Send>() {}
        check_traits::<IoStream>();
    }
}
