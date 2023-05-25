use std::fmt;
use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use std::sync::Arc;

use log::warn;
use tokio::time::Sleep;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use hyper::server::accept::Accept;
use state::InitCell;

pub use tokio::net::TcpListener;

/// A thin wrapper over raw, DER-encoded X.509 client certificate data.
// NOTE: `rustls::Certificate` is exactly isomorphic to `CertificateData`.
#[doc(inline)]
#[cfg(feature = "tls")]
pub use rustls::Certificate as CertificateData;

/// A thin wrapper over raw, DER-encoded X.509 client certificate data.
#[cfg(not(feature = "tls"))]
#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CertificateData(pub Vec<u8>);

/// A collection of raw certificate data.
#[derive(Clone, Default)]
pub struct Certificates(Arc<InitCell<Vec<CertificateData>>>);

impl From<Vec<CertificateData>> for Certificates {
    fn from(value: Vec<CertificateData>) -> Self {
        Certificates(Arc::new(value.into()))
    }
}

impl Certificates {
    /// Set the the raw certificate chain data. Only the first call actually
    /// sets the data; the remaining do nothing.
    #[cfg(feature = "tls")]
    pub(crate) fn set(&self, data: Vec<CertificateData>) {
        self.0.set(data);
    }

    /// Returns the raw certificate chain data, if any is available.
    pub fn chain_data(&self) -> Option<&[CertificateData]> {
        self.0.try_get().map(|v| v.as_slice())
    }
}

// TODO.async: 'Listener' and 'Connection' provide common enough functionality
// that they could be introduced in upstream libraries.
/// A 'Listener' yields incoming connections
pub trait Listener {
    /// The connection type returned by this listener.
    type Connection: Connection;

    /// Return the actual address this listener bound to.
    fn local_addr(&self) -> Option<SocketAddr>;

    /// Try to accept an incoming Connection if ready. This should only return
    /// an `Err` when a fatal problem occurs as Hyper kills the server on `Err`.
    fn poll_accept(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>
    ) -> Poll<io::Result<Self::Connection>>;
}

/// A 'Connection' represents an open connection to a client
pub trait Connection: AsyncRead + AsyncWrite {
    /// The remote address, i.e. the client's socket address, if it is known.
    fn peer_address(&self) -> Option<SocketAddr>;

    /// Requests that the connection not delay reading or writing data as much
    /// as possible. For connections backed by TCP, this corresponds to setting
    /// `TCP_NODELAY`.
    fn enable_nodelay(&self) -> io::Result<()>;

    /// DER-encoded X.509 certificate chain presented by the client, if any.
    ///
    /// The certificate order must be as it appears in the TLS protocol: the
    /// first certificate relates to the peer, the second certifies the first,
    /// the third certifies the second, and so on.
    ///
    /// Defaults to an empty vector to indicate that no certificates were
    /// presented.
    fn peer_certificates(&self) -> Option<Certificates> { None }
}

pin_project_lite::pin_project! {
    /// This is a generic version of hyper's AddrIncoming that is intended to be
    /// usable with listeners other than a plain TCP stream, e.g. TLS and/or Unix
    /// sockets. It does so by bridging the `Listener` trait to what hyper wants (an
    /// Accept). This type is internal to Rocket.
    #[must_use = "streams do nothing unless polled"]
    pub struct Incoming<L> {
        sleep_on_errors: Option<Duration>,
        nodelay: bool,
        #[pin]
        pending_error_delay: Option<Sleep>,
        #[pin]
        listener: L,
    }
}

impl<L: Listener> Incoming<L> {
    /// Construct an `Incoming` from an existing `Listener`.
    pub fn new(listener: L) -> Self {
        Self {
            listener,
            sleep_on_errors: Some(Duration::from_millis(250)),
            pending_error_delay: None,
            nodelay: false,
        }
    }

    /// Set whether and how long to sleep on accept errors.
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
    pub fn sleep_on_errors(mut self, val: Option<Duration>) -> Self {
        self.sleep_on_errors = val;
        self
    }

    /// Set whether to request no delay on all incoming connections. The default
    /// is `false`. See [`Connection::enable_nodelay()`] for details.
    pub fn nodelay(mut self, nodelay: bool) -> Self {
        self.nodelay = nodelay;
        self
    }

    fn poll_accept_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>
    ) -> Poll<io::Result<L::Connection>> {
        /// This function defines per-connection errors: errors that affect only
        /// a single connection's accept() and don't imply anything about the
        /// success probability of the next accept(). Thus, we can attempt to
        /// `accept()` another connection immediately. All other errors will
        /// incur a delay before the next `accept()` is performed. The delay is
        /// useful to handle resource exhaustion errors like ENFILE and EMFILE.
        /// Otherwise, could enter into tight loop.
        fn is_connection_error(e: &io::Error) -> bool {
            matches!(e.kind(),
                | io::ErrorKind::ConnectionRefused
                | io::ErrorKind::ConnectionAborted
                | io::ErrorKind::ConnectionReset)
        }

        let mut this = self.project();
        loop {
            // Check if a previous sleep timer is active, set on I/O errors.
            if let Some(delay) = this.pending_error_delay.as_mut().as_pin_mut() {
                futures::ready!(delay.poll(cx));
            }

            this.pending_error_delay.set(None);

            match futures::ready!(this.listener.as_mut().poll_accept(cx)) {
                Ok(stream) => {
                    if *this.nodelay {
                        if let Err(e) = stream.enable_nodelay() {
                            warn!("failed to enable NODELAY: {}", e);
                        }
                    }

                    return Poll::Ready(Ok(stream));
                },
                Err(e) => {
                    if is_connection_error(&e) {
                        warn!("single connection accept error {}; accepting next now", e);
                    } else if let Some(duration) = this.sleep_on_errors {
                        // We might be able to recover. Try again in a bit.
                        warn!("accept error {}; recovery attempt in {}ms", e, duration.as_millis());
                        this.pending_error_delay.set(Some(tokio::time::sleep(*duration)));
                    } else {
                        return Poll::Ready(Err(e));
                    }
                },
            }
        }
    }
}

impl<L: Listener> Accept for Incoming<L> {
    type Conn = L::Connection;
    type Error = io::Error;

    #[inline]
    fn poll_accept(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>
    ) -> Poll<Option<io::Result<Self::Conn>>> {
        self.poll_accept_next(cx).map(Some)
    }
}

impl<L: fmt::Debug> fmt::Debug for Incoming<L> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Incoming")
            .field("listener", &self.listener)
            .finish()
    }
}

impl Listener for TcpListener {
    type Connection = TcpStream;

    #[inline]
    fn local_addr(&self) -> Option<SocketAddr> {
        self.local_addr().ok()
    }

    #[inline]
    fn poll_accept(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>
    ) -> Poll<io::Result<Self::Connection>> {
        (*self).poll_accept(cx).map_ok(|(stream, _addr)| stream)
    }
}

impl Connection for TcpStream {
    #[inline]
    fn peer_address(&self) -> Option<SocketAddr> {
        self.peer_addr().ok()
    }

    #[inline]
    fn enable_nodelay(&self) -> io::Result<()> {
        self.set_nodelay(true)
    }
}
