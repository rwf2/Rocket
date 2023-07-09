use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::mpsc;
use std::sync::{Arc, Mutex, RwLock};
use std::task::{Context, Poll};
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::watch;

use rustls::server::ClientHello;
use rustls::{cipher_suite, sign::CertifiedKey, PrivateKey};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{server::TlsStream as BareTlsStream, Accept, TlsAcceptor};

use crate::listener::{Certificates, Connection, Listener};
use crate::tls::util::{load_ca_certs, load_certs, load_private_key};
use rustls::Certificate;

pub struct Resolver {
    cert_chain: Vec<Certificate>,
    private_key: PrivateKey,
}

/// A TLS listener over TCP.
pub struct TlsListener {
    listener: TcpListener,
    acceptor: TlsAcceptor,
}

/// This implementation exists so that ROCKET_WORKERS=1 can make progress while
/// a TLS handshake is being completed. It does this by returning `Ready` from
/// `poll_accept()` as soon as we have a TCP connection and performing the
/// handshake in the `AsyncRead` and `AsyncWrite` implementations.
///
/// A straight-forward implementation of this strategy results in none of the
/// TLS information being available at the time the connection is "established",
/// that is, when `poll_accept()` returns, since the handshake has yet to occur.
/// Importantly, certificate information isn't available at the time that we
/// request it.
///
/// The underlying problem is e hyper's "Accept" trait. Were we to manage
/// connections ourselves, we'd likely want to:
///
///   1. Stop blocking the worker as soon as we have a TCP connection.
///   2. Perform the handshake in the background.
///   3. Give the connection to Rocket when/if the handshake is done.
///
/// See hyperium/hyper/issues/2321 for more details.
///
/// To work around this, we "lie" when `peer_certificates()` are requested and
/// always return `Some(Certificates)`. Internally, `Certificates` is an
/// `Arc<InitCell<Vec<CertificateData>>>`, effectively a shared, thread-safe,
/// `OnceCell`. The cell is initially empty and is filled as soon as the
/// handshake is complete. If the certificate data were to be requested prior to
/// this point, it would be empty. However, in Rocket, we only request
/// certificate data when we have a `Request` object, which implies we're receiving payload data, which implies the TLS handshake has finished, so the
/// certificate data as seen by a Rocket application will always be "fresh".
pub struct TlsStream {
    remote: SocketAddr,
    state: TlsState,
    certs: Certificates,
}

/// State of `TlsStream`.
pub enum TlsState {
    /// The TLS handshake is taking place. We don't have a full connection yet.
    Handshaking(Accept<TcpStream>),
    /// TLS handshake completed successfully; we're getting payload data.
    Streaming(BareTlsStream<TcpStream>),
}

/// TLS as ~configured by `TlsConfig` in `rocket` core.
pub struct Config<R>
where
    R: io::BufRead + std::marker::Send + std::marker::Sync + 'static,
{
    pub cert_chain: R,
    pub private_key: R,
    pub ciphersuites: Vec<rustls::SupportedCipherSuite>,
    pub prefer_server_order: bool,
    pub ca_certs: Option<R>,
    pub mandatory_mtls: bool,
}

impl rustls::server::ResolvesServerCert for Resolver {
    fn resolve(&self, _client_hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        let sign_key = rustls::sign::any_supported_type(&self.private_key).unwrap();
        let cert = Arc::new(CertifiedKey::new(self.cert_chain.clone(), sign_key));
        Some(cert)
    }
}

impl Resolver {
    pub fn new<R>(mut c: Arc<RwLock<Config<R>>>) -> Result<Self, Box<dyn std::error::Error>>
    where
        R: io::BufRead + std::marker::Send + std::marker::Sync + 'static,
    {
        let (ctx, crx) = mpsc::channel::<Vec<Certificate>>();
        let (ktx, krx) = mpsc::channel::<PrivateKey>();

        let mut stream = signal(SignalKind::user_defined1())?;

        tokio::spawn(async move {
            loop {
                stream.recv().await;
                let mut config = c.write().unwrap();
                let cert_chain = load_certs(&mut config.cert_chain)
                    .map_err(|e| io::Error::new(e.kind(), format!("bad TLS cert chain: {}", e)))
                    .unwrap();

                let private_key = load_private_key(&mut config.private_key)
                    .map_err(|e| io::Error::new(e.kind(), format!("bad TLS private key: {}", e)))
                    .unwrap();

                ctx.send(cert_chain).unwrap();
                ktx.send(private_key).unwrap();
            }
        });

        let cert_chain = crx.recv().unwrap();
        let private_key = krx.recv().unwrap();

        Ok(Self {
            cert_chain,
            private_key,
        })
    }
}

impl TlsListener {
    pub async fn bind<R>(addr: SocketAddr, mut c: Config<R>) -> io::Result<TlsListener>
    where
        R: io::BufRead + std::marker::Send + std::marker::Sync + 'static,
    {
        use rustls::server::{AllowAnyAnonymousOrAuthenticatedClient, AllowAnyAuthenticatedClient};
        use rustls::server::{NoClientAuth, ServerConfig, ServerSessionMemoryCache};

        let client_auth = match c.ca_certs {
            Some(ref mut ca_certs) => match load_ca_certs(ca_certs) {
                Ok(ca) if c.mandatory_mtls => AllowAnyAuthenticatedClient::new(ca).boxed(),
                Ok(ca) => AllowAnyAnonymousOrAuthenticatedClient::new(ca).boxed(),
                Err(e) => return Err(io::Error::new(e.kind(), format!("bad CA cert(s): {}", e))),
            },
            None => NoClientAuth::boxed(),
        };

        let cipher_suite = &c.ciphersuites.to_vec();

        let prefer_server_order = c.prefer_server_order;

        println!("Getting here");

        let config = Arc::new(RwLock::new(c));

        let resolver = Resolver::new(Arc::clone(&config)).unwrap();

        let mut tls_config = ServerConfig::builder()
            .with_cipher_suites(cipher_suite)
            .with_safe_default_kx_groups()
            .with_safe_default_protocol_versions()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("bad TLS config: {}", e)))?
            .with_client_cert_verifier(client_auth)
            .with_cert_resolver(Arc::new(resolver));

        tls_config.ignore_client_order = prefer_server_order;

        tls_config.alpn_protocols = vec![b"http/1.1".to_vec()];
        if cfg!(feature = "http2") {
            tls_config.alpn_protocols.insert(0, b"h2".to_vec());
        }

        tls_config.session_storage = ServerSessionMemoryCache::new(1024);
        tls_config.ticketer = rustls::Ticketer::new().map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("bad TLS ticketer: {}", e))
        })?;

        let listener = TcpListener::bind(addr).await?;
        let acceptor = TlsAcceptor::from(Arc::new(tls_config));
        Ok(TlsListener { listener, acceptor })
    }
}

impl Listener for TlsListener {
    type Connection = TlsStream;

    fn local_addr(&self) -> Option<SocketAddr> {
        self.listener.local_addr().ok()
    }

    fn poll_accept(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<Self::Connection>> {
        match futures::ready!(self.listener.poll_accept(cx)) {
            Ok((io, addr)) => Poll::Ready(Ok(TlsStream {
                remote: addr,
                state: TlsState::Handshaking(self.acceptor.accept(io)),
                // These are empty and filled in after handshake is complete.
                certs: Certificates::default(),
            })),
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

impl Connection for TlsStream {
    fn peer_address(&self) -> Option<SocketAddr> {
        Some(self.remote)
    }

    fn enable_nodelay(&self) -> io::Result<()> {
        // If `Handshaking` is `None`, it either failed, so we returned an `Err`
        // from `poll_accept()` and there's no connection to enable `NODELAY`
        // on, or it succeeded, so we're in the `Streaming` stage and we have
        // infallible access to the connection.
        match &self.state {
            TlsState::Handshaking(accept) => match accept.get_ref() {
                None => Ok(()),
                Some(s) => s.enable_nodelay(),
            },
            TlsState::Streaming(stream) => stream.get_ref().0.enable_nodelay(),
        }
    }

    fn peer_certificates(&self) -> Option<Certificates> {
        Some(self.certs.clone())
    }
}

impl TlsStream {
    fn poll_accept_then<F, T>(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut f: F,
    ) -> Poll<io::Result<T>>
    where
        F: FnMut(&mut BareTlsStream<TcpStream>, &mut Context<'_>) -> Poll<io::Result<T>>,
    {
        loop {
            match self.state {
                TlsState::Handshaking(ref mut accept) => {
                    match futures::ready!(Pin::new(accept).poll(cx)) {
                        Ok(stream) => {
                            if let Some(cert_chain) = stream.get_ref().1.peer_certificates() {
                                self.certs.set(cert_chain.to_vec());
                            }

                            self.state = TlsState::Streaming(stream);
                        }
                        Err(e) => {
                            log::warn!("tls handshake with {} failed: {}", self.remote, e);
                            return Poll::Ready(Err(e));
                        }
                    }
                }
                TlsState::Streaming(ref mut stream) => return f(stream, cx),
            }
        }
    }
}

impl AsyncRead for TlsStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        self.poll_accept_then(cx, |stream, cx| Pin::new(stream).poll_read(cx, buf))
    }
}

impl AsyncWrite for TlsStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.poll_accept_then(cx, |stream, cx| Pin::new(stream).poll_write(cx, buf))
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut self.state {
            TlsState::Handshaking(accept) => match accept.get_mut() {
                Some(io) => Pin::new(io).poll_flush(cx),
                None => Poll::Ready(Ok(())),
            },
            TlsState::Streaming(stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut self.state {
            TlsState::Handshaking(accept) => match accept.get_mut() {
                Some(io) => Pin::new(io).poll_shutdown(cx),
                None => Poll::Ready(Ok(())),
            },
            TlsState::Streaming(stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}
