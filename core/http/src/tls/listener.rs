use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::net::SocketAddr;
use std::future::Future;

use rustls::{ServerConfig, SupportedCipherSuite};
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
    Accepting(Accept<TcpStream>),
}

pub struct Config<R> {
    pub cert_chain: R,
    pub private_key: R,
    pub ciphersuites: Vec<&'static SupportedCipherSuite>,
    pub prefer_server_order: bool,
    pub ca_certs: Option<R>,
    pub mandatory_mtls: bool,
}

impl TlsListener {
    pub async fn bind<R>(addr: SocketAddr, mut c: Config<R>) -> io::Result<TlsListener>
        where R: io::BufRead
    {
        let cert_chain = load_certs(&mut c.cert_chain).map_err(|e| {
            let msg = format!("malformed TLS certificate chain: {}", e);
            io::Error::new(e.kind(), msg)
        })?;

        let key = load_private_key(&mut c.private_key).map_err(|e| {
            let msg = format!("malformed TLS private key: {}", e);
            io::Error::new(e.kind(), msg)
        })?;

        let client_auth = match c.ca_certs {
            Some(ref mut ca_certs) => {
                let roots = load_ca_certs(ca_certs).map_err(|e| {
                    let msg = format!("malformed CA certificate(s): {}", e);
                    io::Error::new(e.kind(), msg)
                })?;

                if c.mandatory_mtls {
                    rustls::AllowAnyAuthenticatedClient::new(roots)
                } else {
                    rustls::AllowAnyAnonymousOrAuthenticatedClient::new(roots)
                }
            }
            None => rustls::NoClientAuth::new(),
        };

        let mut tls_config = ServerConfig::new(client_auth);
        let cache = rustls::ServerSessionMemoryCache::new(1024);
        tls_config.set_persistence(cache);
        tls_config.ticketer = rustls::Ticketer::new();
        tls_config.ciphersuites = c.ciphersuites;
        tls_config.ignore_client_order = c.prefer_server_order;
        tls_config.set_single_cert(cert_chain, key).expect("invalid key");
        tls_config.set_protocols(&[b"h2".to_vec(), b"http/1.1".to_vec()]);

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
            match self.state {
                State::Listening => {
                    match self.listener.poll_accept(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                        Poll::Ready(Ok((stream, _addr))) => {
                            let fut = self.acceptor.accept(stream);
                            self.state = State::Accepting(fut);
                        }
                    }
                }
                State::Accepting(ref mut fut) => {
                    match Pin::new(fut).poll(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(result) => {
                            self.state = State::Listening;
                            return Poll::Ready(result);
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

    fn peer_certificates(&self) -> Option<Vec<RawCertificate>> {
        use rustls::Session;

        self.get_ref().1.get_peer_certificates()
    }
}
