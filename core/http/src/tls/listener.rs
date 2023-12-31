use std::io;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::future::Future;
use std::net::SocketAddr;

use rustls::sign::CertifiedKey;
use rustls::server::{ServerSessionMemoryCache, ServerConfig, WebPkiClientVerifier};

use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_rustls::{Accept, TlsAcceptor, server::TlsStream as BareTlsStream};


use crate::tls::util::{load_cert_chain, load_key, load_ca_certs};
use crate::listener::{Connection, Listener, Certificates};

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
/// The underlying problem is hyper's "Accept" trait. Were we to manage
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
/// `Arc<InitCell<Vec<CertificateDer>>>`, effectively a shared, thread-safe,
/// `OnceCell`. The cell is initially empty and is filled as soon as the
/// handshake is complete. If the certificate data were to be requested prior to
/// this point, it would be empty. However, in Rocket, we only request
/// certificate data when we have a `Request` object, which implies we're
/// receiving payload data, which implies the TLS handshake has finished, so the
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

#[derive(Clone)]
pub enum FileOrBytes {
    File(PathBuf),
    Bytes(Vec<u8>),
}

/// TLS as ~configured by `TlsConfig` in `rocket` core.
pub struct Config<R> {
    //pub cert_chain: R,
    //pub private_key: R,
    pub cert_chain: FileOrBytes,
    pub private_key: FileOrBytes,
    pub ciphersuites: Vec<rustls::SupportedCipherSuite>,
    pub prefer_server_order: bool,
    pub ca_certs: Option<R>,
    pub mandatory_mtls: bool,
    pub tls_updater: Option<std::sync::Arc<std::sync::RwLock<DynamicConfig>>>,
}

#[derive(Debug, Default)]
pub struct DynamicConfig {
    pub certs: Vec<u8>,
    pub key: Vec<u8>,
}

type Reader = Box<dyn std::io::BufRead + Sync + Send>;

fn to_reader(value: &FileOrBytes) -> io::Result<Reader> {
    match value {
        FileOrBytes::File(path) => {
            let file = std::fs::File::open(&path).map_err(move |e| {
                let msg = format!("error reading TLS file `{}`", e);
                std::io::Error::new(e.kind(), msg)
            })?;

            Ok(Box::new(io::BufReader::new(file)))
        }
        FileOrBytes::Bytes(vec) => Ok(Box::new(io::Cursor::new(vec.clone()))),
    }
}

#[derive(Debug)]
pub struct CertResolver {
    pub certified_key: Arc<std::sync::RwLock<Option<Arc<CertifiedKey>>>>,
    _handle: tokio::task::JoinHandle<()>,

}

impl CertResolver {
    pub async fn new<R>(config: &Config<R>) -> crate::tls::Result<Arc<Self>>
        where R: io::BufRead 
    { 

        let certified_key = Arc::new(std::sync::RwLock::new(None));

        let private_key = config.private_key.to_owned();
        let cert_chain = config.cert_chain.to_owned();

        let loop_mutex = certified_key.clone();
        let loop_updater = config.tls_updater.as_ref().map(|i| i.clone());

        let handle = tokio::spawn(async move {
            
            loop {
                if let Ok(mut certified_key) = loop_mutex.write() {
                    let key = load_key(&mut to_reader(&private_key).unwrap()).unwrap();
                    let cert_chain = load_cert_chain(&mut to_reader(&cert_chain).unwrap()).unwrap();
            
                    *certified_key = Some(Arc::new(CertifiedKey::new(
                        cert_chain, 
                        rustls::crypto::ring::sign::any_supported_type(&key).unwrap()
                    )));
                }

                if let Some(loop_updater) = loop_updater.clone() {
                    dbg!(loop_updater.read());
                }
                
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            }
        });


        Ok(Arc::new(Self {
            certified_key: certified_key,
            _handle: handle,

        }))
    }
}

impl rustls::server::ResolvesServerCert for CertResolver {
    fn resolve(&self, _client_hello: rustls::server::ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        let cert = self.certified_key.read().unwrap();
        if cert.is_none() { return None; }
        Some(cert.as_ref().unwrap().clone())
    }
}

impl TlsListener {
    pub async fn bind<R, T>(addr: SocketAddr, mut c: Config<R>, cert_resolver: Arc<T>) -> crate::tls::Result<TlsListener>
        where R: io::BufRead, T: rustls::server::ResolvesServerCert + 'static
    {
        let provider = rustls::crypto::CryptoProvider {
            cipher_suites: c.ciphersuites,
            ..rustls::crypto::ring::default_provider()
        };

        let verifier = match c.ca_certs {
            Some(ref mut ca_certs) => {
                let ca_roots = Arc::new(load_ca_certs(ca_certs)?);
                let verifier = WebPkiClientVerifier::builder(ca_roots);
                match c.mandatory_mtls {
                    true => verifier.build()?,
                    false => verifier.allow_unauthenticated().build()?,
                }
            },
            None => WebPkiClientVerifier::no_client_auth(),
        };

        let mut config = ServerConfig::builder_with_provider(Arc::new(provider))
            .with_safe_default_protocol_versions()?
            .with_client_cert_verifier(verifier)
            .with_cert_resolver(cert_resolver);

        config.ignore_client_order = c.prefer_server_order;
        config.session_storage = ServerSessionMemoryCache::new(1024);
        config.ticketer = rustls::crypto::ring::Ticketer::new()?;
        config.alpn_protocols = vec![b"http/1.1".to_vec()];
        if cfg!(feature = "http2") {
            config.alpn_protocols.insert(0, b"h2".to_vec());
        }

        let listener = TcpListener::bind(addr).await?;
        let acceptor = TlsAcceptor::from(Arc::new(config));
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
        cx: &mut Context<'_>
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
            TlsState::Streaming(stream) => stream.get_ref().0.enable_nodelay()
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
        mut f: F
    ) -> Poll<io::Result<T>>
        where F: FnMut(&mut BareTlsStream<TcpStream>, &mut Context<'_>) -> Poll<io::Result<T>>
    {
        loop {
            match self.state {
                TlsState::Handshaking(ref mut accept) => {
                    match futures::ready!(Pin::new(accept).poll(cx)) {
                        Ok(stream) => {
                            if let Some(peer_certs) = stream.get_ref().1.peer_certificates() {
                                self.certs.set(peer_certs.into_iter()
                                    .map(|v| crate::listener::CertificateDer(v.clone().into_owned()))
                                    .collect());
                            }

                            self.state = TlsState::Streaming(stream);
                        }
                        Err(e) => {
                            log::warn!("tls handshake with {} failed: {}", self.remote, e);
                            return Poll::Ready(Err(e));
                        }
                    }
                },
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
            }
            TlsState::Streaming(stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut self.state {
            TlsState::Handshaking(accept) => match accept.get_mut() {
                Some(io) => Pin::new(io).poll_shutdown(cx),
                None => Poll::Ready(Ok(())),
            }
            TlsState::Streaming(stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}
