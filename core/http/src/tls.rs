use std::fs;
use std::future::Future;
use std::io::{self, BufReader, Error, ErrorKind};
use std::net::SocketAddr;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use tokio::net::{TcpListener, TcpStream};

use tokio_rustls::{TlsAcceptor, server::TlsStream};
use tokio_rustls::rustls;

pub use rustls::internal::pemfile;
pub use rustls::{Certificate, PrivateKey, ServerConfig};

use crate::listener::{Connection, Listener};

// TODO.async: consider using async fs operations
pub fn load_certs<P: AsRef<Path>>(path: P) -> Result<Vec<rustls::Certificate>, Error> {
    let certfile = fs::File::open(path.as_ref())?;
    let mut reader = BufReader::new(certfile);
    pemfile::certs(&mut reader).map_err(|_| Error::from(ErrorKind::InvalidData))
}

pub fn load_private_key<P: AsRef<Path>>(path: P) -> Result<rustls::PrivateKey, Error> {
    use std::io::Seek;
    use std::io::BufRead;

    let keyfile = fs::File::open(path.as_ref())?;
    let mut reader = BufReader::new(keyfile);

    // "rsa" (PKCS1) PEM files have a different first-line header than PKCS8
    // PEM files, use that to determine the parse function to use.
    let mut first_line = String::new();
    reader.read_line(&mut first_line)?;
    reader.seek(io::SeekFrom::Start(0))?;

    let private_keys_fn = match first_line.trim_end() {
        "-----BEGIN RSA PRIVATE KEY-----" => pemfile::rsa_private_keys,
        "-----BEGIN PRIVATE KEY-----" => pemfile::pkcs8_private_keys,
        _ => return Err(Error::from(ErrorKind::InvalidData)),
    };

    let key = private_keys_fn(&mut reader)
        .map_err(|_| Error::from(ErrorKind::InvalidData))
        .and_then(|mut keys| match keys.len() {
            1 => Ok(keys.remove(0)),
            _ => Err(Error::from(ErrorKind::InvalidData)),
        })?;

    // Ensure we can use the key.
    if rustls::sign::RSASigningKey::new(&key).is_err() {
        Err(Error::from(ErrorKind::InvalidData))
    } else {
        Ok(key)
    }
}

pub struct TlsListener {
    listener: TcpListener,
    acceptor: TlsAcceptor,
    state: TlsListenerState,
}

enum TlsListenerState {
    Listening,
    Accepting(Pin<Box<dyn Future<Output=Result<TlsStream<TcpStream>, Error>> + Send>>),
}

impl Listener for TlsListener {
    type Connection = TlsStream<TcpStream>;

    fn local_addr(&self) -> Option<SocketAddr> {
        self.listener.local_addr().ok()
    }

    fn poll_accept(&mut self, cx: &mut Context<'_>) -> Poll<Result<Self::Connection, Error>> {
        loop {
            match &mut self.state {
                TlsListenerState::Listening => {
                    match self.listener.poll_accept(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Ok((stream, _addr))) => {
                            self.state = TlsListenerState::Accepting(Box::pin(self.acceptor.accept(stream)));
                        }
                        Poll::Ready(Err(e)) => {
                            return Poll::Ready(Err(e));
                        }
                    }
                }
                TlsListenerState::Accepting(fut) => {
                    match fut.as_mut().poll(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(result) => {
                            self.state = TlsListenerState::Listening;
                            return Poll::Ready(result);
                        }
                    }
                }
            }
        }
    }
}

pub async fn bind_tls<P: AsRef<Path>>(
    address: SocketAddr,
    cert_path: P,
    key_path: P
) -> Result<TlsListener, Error> {
    let cert_chain = load_certs(cert_path)?;
    let key = load_private_key(key_path)?;
    let listener = TcpListener::bind(address).await?;

    let client_auth = rustls::NoClientAuth::new();
    let mut tls_config = ServerConfig::new(client_auth);
    let cache = rustls::ServerSessionMemoryCache::new(1024);
    tls_config.set_persistence(cache);
    tls_config.ticketer = rustls::Ticketer::new();
    tls_config.set_single_cert(cert_chain, key).expect("invalid key");

    let acceptor = TlsAcceptor::from(Arc::new(tls_config));
    let state = TlsListenerState::Listening;

    Ok(TlsListener { listener, acceptor, state })
}

impl Connection for TlsStream<TcpStream> {
    fn remote_addr(&self) -> Option<SocketAddr> {
        self.get_ref().0.remote_addr()
    }
}
