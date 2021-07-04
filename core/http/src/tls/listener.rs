use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::net::SocketAddr;
use std::future::Future;

use rustls::{ServerConfig, SupportedCipherSuite};
use tokio_rustls::{TlsAcceptor, Accept, server::TlsStream};
use tokio::net::{TcpListener, TcpStream};

use crate::tls::util::{load_certs, load_private_key};
use crate::listener::{Connection, Listener};

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

impl TlsListener {
    pub async fn bind(
        address: SocketAddr,
        mut cert_chain: impl io::BufRead + Send,
        mut private_key: impl io::BufRead + Send,
        ciphersuites: impl Iterator<Item = &'static SupportedCipherSuite>,
        prefer_server_order: bool,
    ) -> io::Result<TlsListener> {
        let cert_chain = load_certs(&mut cert_chain).map_err(|e| {
            let msg = format!("malformed TLS certificate chain: {}", e);
            io::Error::new(e.kind(), msg)
        })?;

        let key = load_private_key(&mut private_key).map_err(|e| {
            let msg = format!("malformed TLS private key: {}", e);
            io::Error::new(e.kind(), msg)
        })?;

        let client_auth = rustls::NoClientAuth::new();
        let mut tls_config = ServerConfig::new(client_auth);
        let cache = rustls::ServerSessionMemoryCache::new(1024);
        tls_config.set_persistence(cache);
        tls_config.ticketer = rustls::Ticketer::new();
        tls_config.ciphersuites = ciphersuites.collect();
        tls_config.ignore_client_order = prefer_server_order;
        tls_config.set_single_cert(cert_chain, key).expect("invalid key");
        tls_config.set_protocols(&[b"h2".to_vec(), b"http/1.1".to_vec()]);

        let listener = TcpListener::bind(address).await?;
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
}
