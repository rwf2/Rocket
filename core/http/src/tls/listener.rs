use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::net::SocketAddr;
use std::future::Future;

use futures::ready;
use tokio_rustls::{TlsAcceptor, Accept, server::TlsStream};
use tokio::net::{TcpListener, TcpStream};

use crate::tls::util::{load_certs, load_private_key, load_ca_certs};
use crate::listener::{Connection, Listener, RawCertificate};

/// A TLS listener over TCP.
pub struct TlsListener {
    listener: TcpListener,
    acceptor: TlsAcceptor,
    state: State,
}

enum State {
    Listening,
    Accepting(Accept<TcpStream>, SocketAddr),
}

pub struct Config<R> {
    pub cert_chain: R,
    pub private_key: R,
    pub ciphersuites: Vec<rustls::SupportedCipherSuite>,
    pub prefer_server_order: bool,
    pub ca_certs: Option<R>,
    pub mandatory_mtls: bool,
}

impl TlsListener {
    pub async fn bind<R>(addr: SocketAddr, mut c: Config<R>) -> io::Result<TlsListener>
        where R: io::BufRead
    {
        use rustls::server::{AllowAnyAuthenticatedClient, AllowAnyAnonymousOrAuthenticatedClient};
        use rustls::server::{NoClientAuth, ServerSessionMemoryCache, ServerConfig};

        let cert_chain = load_certs(&mut c.cert_chain)
            .map_err(|e| io::Error::new(e.kind(), format!("bad TLS cert chain: {}", e)))?;

        let key = load_private_key(&mut c.private_key)
            .map_err(|e| io::Error::new(e.kind(), format!("bad TLS private key: {}", e)))?;

        let client_auth = match c.ca_certs {
            Some(ref mut ca_certs) => match load_ca_certs(ca_certs) {
                Ok(ca_roots) if c.mandatory_mtls => AllowAnyAuthenticatedClient::new(ca_roots),
                Ok(ca_roots) => AllowAnyAnonymousOrAuthenticatedClient::new(ca_roots),
                Err(e) => return Err(io::Error::new(e.kind(), format!("bad CA cert(s): {}", e))),
            },
            None => NoClientAuth::new(),
        };

        let mut tls_config = ServerConfig::builder()
            .with_cipher_suites(&c.ciphersuites)
            .with_safe_default_kx_groups()
            .with_safe_default_protocol_versions()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("bad TLS config: {}", e)))?
            .with_client_cert_verifier(client_auth)
            .with_single_cert(cert_chain, key)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("bad TLS config: {}", e)))?;

        tls_config.ignore_client_order = c.prefer_server_order;

        tls_config.alpn_protocols = vec![b"http/1.1".to_vec()];
        if cfg!(feature = "http2") {
            tls_config.alpn_protocols.insert(0, b"h2".to_vec());
        }

        tls_config.session_storage = ServerSessionMemoryCache::new(1024);
        tls_config.ticketer = rustls::Ticketer::new()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("bad TLS ticketer: {}", e)))?;

        let listener = TcpListener::bind(addr).await?;
        let acceptor = TlsAcceptor::from(Arc::new(tls_config));
        Ok(TlsListener { listener, acceptor, state: State::Listening })
    }
}

impl Listener for TlsListener {
    type Connection = TlsStream<TcpStream>;

    fn local_addr(&self) -> Option<SocketAddr> {
        self.listener.local_addr().ok()
    }

    fn poll_accept(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>
    ) -> Poll<io::Result<Self::Connection>> {
        loop {
            match &mut self.state {
                State::Listening => {
                    match ready!(self.listener.poll_accept(cx)) {
                        Err(e) => return Poll::Ready(Err(e)),
                        Ok((stream, addr)) => {
                            let accept = self.acceptor.accept(stream);
                            self.state = State::Accepting(accept, addr);
                        }
                    }
                }
                State::Accepting(accept, addr) => {
                    match ready!(Pin::new(accept).poll(cx)) {
                        Ok(stream) => {
                            self.state = State::Listening;
                            return Poll::Ready(Ok(stream));
                        },
                        Err(e) => {
                            log::warn!("TLS accept {} failure: {}", addr, e);
                            self.state = State::Listening;
                        }
                    }
                }
            }
        }
    }
}

impl Connection for TlsStream<TcpStream> {
    fn peer_address(&self) -> Option<SocketAddr> {
        self.get_ref().0.peer_address()
    }

    fn peer_certificates(&self) -> Option<&[RawCertificate]> {
        self.get_ref().1.peer_certificates()
    }

    fn enable_nodelay(&self) -> io::Result<()> {
        self.get_ref().0.enable_nodelay()
    }
}
