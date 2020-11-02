use std::fmt;
use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use hyper::server::accept::Accept;

use log::{debug, error};

use tokio::time::Delay;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};

// TODO.async: 'Listener' and 'Connection' provide common enough functionality
// that they could be introduced in upstream libraries.
/// A 'Listener' yields incoming connections
pub trait Listener {
    type Connection: Connection;

    /// Return the actual address this listener bound to.
    fn local_addr(&self) -> Option<SocketAddr>;

    /// Try to accept an incoming Connection if ready
    fn poll_accept(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<Self::Connection>>;
}

/// A 'Connection' represents an open connection to a client
pub trait Connection: AsyncRead + AsyncWrite {
    fn remote_addr(&self) -> Option<SocketAddr>;
}

/// This is a generic version of hyper's AddrIncoming that is intended to be
/// usable with listeners other than a plain TCP stream, e.g. TLS and/or Unix
/// sockets. It does so by bridging the `Listener` trait to what hyper wants (an
/// Accept). This type is internal to Rocket.
#[must_use = "streams do nothing unless polled"]
pub struct Incoming<L> {
    listener: L,
    sleep_on_errors: Option<Duration>,
    pending_error_delay: Option<Delay>,
}

impl<L: Listener> Incoming<L> {
    /// Construct an `Incoming` from an existing `Listener`.
    pub fn from_listener(listener: L) -> Self {
        Self {
            listener,
            sleep_on_errors: Some(Duration::from_secs(1)),
            pending_error_delay: None,
        }
    }

    /// Set whether to sleep on accept errors.
    ///
    /// A possible scenario is that the process has hit the max open files
    /// allowed, and so trying to accept a new connection will fail with
    /// `EMFILE`. In some cases, it's preferable to just wait for some time, if
    /// the application will likely close some files (or connections), and try
    /// to accept the connection again. If this option is `true`, the error
    /// will be logged at the `error` level, since it is still a big deal,
    /// and then the listener will sleep for 1 second.
    ///
    /// In other cases, hitting the max open files should be treat similarly
    /// to being out-of-memory, and simply error (and shutdown). Setting
    /// this option to `None` will allow that.
    ///
    /// Default is 1 second.
    pub fn set_sleep_on_errors(&mut self, val: Option<Duration>) {
        self.sleep_on_errors = val;
    }

    fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<L::Connection>> {
        // Check if a previous delay is active that was set by IO errors.
        if let Some(ref mut delay) = self.pending_error_delay {
            match Pin::new(delay).poll(cx) {
                Poll::Ready(()) => {}
                Poll::Pending => return Poll::Pending,
            }
        }
        self.pending_error_delay = None;

        loop {
            match self.listener.poll_accept(cx) {
                Poll::Ready(Ok(stream)) => {
                    return Poll::Ready(Ok(stream));
                },
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Err(e)) => {
                    // Connection errors can be ignored directly, continue by
                    // accepting the next request.
                    if is_connection_error(&e) {
                        debug!("accepted connection already errored: {}", e);
                        continue;
                    }

                    if let Some(duration) = self.sleep_on_errors {
                        error!("accept error: {}", e);

                        // Sleep for the specified duration
                        let mut error_delay = tokio::time::delay_for(duration);

                        match Pin::new(&mut error_delay).poll(cx) {
                            Poll::Ready(()) => {
                                // Wow, it's been a second already? Ok then...
                                continue
                            },
                            Poll::Pending => {
                                self.pending_error_delay = Some(error_delay);
                                return Poll::Pending;
                            },
                        }
                    } else {
                        return Poll::Ready(Err(e));
                    }
                },
            }
        }
    }
}

impl<L: Listener + Unpin> Accept for Incoming<L> {
    type Conn = L::Connection;
    type Error = io::Error;

    fn poll_accept(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>
    ) -> Poll<Option<io::Result<Self::Conn>>> {
        self.poll_next(cx).map(Some)
    }
}

/// This function defines errors that are per-connection. Which basically
/// means that if we get this error from `accept()` system call it means
/// next connection might be ready to be accepted.
///
/// All other errors will incur a delay before next `accept()` is performed.
/// The delay is useful to handle resource exhaustion errors like ENFILE
/// and EMFILE. Otherwise, could enter into tight loop.
fn is_connection_error(e: &io::Error) -> bool {
    match e.kind() {
        io::ErrorKind::ConnectionRefused |
        io::ErrorKind::ConnectionAborted |
        io::ErrorKind::ConnectionReset => true,
        _ => false,
    }
}

impl<L: fmt::Debug> fmt::Debug for Incoming<L> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Incoming")
            .field("listener", &self.listener)
            .finish()
    }
}

pub async fn bind_tcp(address: SocketAddr) -> io::Result<TcpListener> {
    Ok(TcpListener::bind(address).await?)
}

impl Listener for TcpListener {
    type Connection = TcpStream;

    fn local_addr(&self) -> Option<SocketAddr> {
        self.local_addr().ok()
    }

    fn poll_accept(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<Self::Connection>> {
        self.poll_accept(cx).map_ok(|(stream, _addr)| stream)
    }
}

impl Connection for TcpStream {
    fn remote_addr(&self) -> Option<SocketAddr> {
        self.peer_addr().ok()
    }
}
