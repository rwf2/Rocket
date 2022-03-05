use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::net::SocketAddr;
use std::future::Future;

use tokio_rustls::{TlsAcceptor, Accept, server::TlsStream};
use tokio::net::{TcpListener, TcpStream};

use crate::tls::util::{load_certs, load_private_key, load_ca_certs};
use crate::listener::{Connection, Listener, RawCertificate};
use crate::http::bindable::BindableAddr;

/// A TLS listener over TCP.
pub struct TlsListener<L: Listener> {
    listener: L,
    acceptor: TlsAcceptor,
    state: State<<L as Listener>::Connection>,
}

enum State<C: Connection> {
    Listening,
    Accepting(Accept<C>),
}

pub struct Config<R> {
    pub cert_chain: R,
    pub private_key: R,
    pub ciphersuites: Vec<rustls::SupportedCipherSuite>,
    pub prefer_server_order: bool,
    pub ca_certs: Option<R>,
    pub mandatory_mtls: bool,
}

impl<L: Listener> TlsListener<L> {
    pub async fn bind<R>(listener: L, mut c: Config<R>) -> io::Result<Self>
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
        tls_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
        tls_config.session_storage = ServerSessionMemoryCache::new(1024);
        tls_config.ticketer = rustls::Ticketer::new()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("bad TLS ticketer: {}", e)))?;

        let acceptor = TlsAcceptor::from(Arc::new(tls_config));
        Ok(TlsListener { listener, acceptor, state: State::<<L as Listener>::Connection>::Listening })
    }
}

impl<C, L> Listener for TlsListener<L>
where
    C: Connection + Unpin,
    L: Listener<Connection = C>,
{
    type Connection = TlsStream<<L as Listener>::Connection>;

    fn local_addr(&self) -> Option<BindableAddr> {
        self.listener.local_addr()
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

impl<C: Connection + Unpin> Connection for TlsStream<C> {
    fn peer_address(&self) -> Option<BindableAddr> {
        self.get_ref().0.peer_address()
    }

    fn peer_certificates(&self) -> Option<&[RawCertificate]> {
        self.get_ref().1.peer_certificates()
    }
}
