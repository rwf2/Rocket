use std::io;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use rustls::internal::pemfile;
use rustls::{Certificate, PrivateKey, ServerConfig};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{TlsAcceptor, Accept, server::TlsStream};
use tokio_rustls::rustls;

use crate::listener::{Connection, Listener};

fn load_certs(reader: &mut dyn io::BufRead) -> io::Result<Vec<Certificate>> {
    pemfile::certs(reader)
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "invalid certificate"))
}

fn load_private_key(reader: &mut dyn io::BufRead) -> io::Result<PrivateKey> {
    use std::io::{Cursor, Error, Read, ErrorKind::Other};

    // "rsa" (PKCS1) PEM files have a different first-line header than PKCS8
    // PEM files, use that to determine the parse function to use.
    let mut first_line = String::new();
    reader.read_line(&mut first_line)?;

    let private_keys_fn = match first_line.trim_end() {
        "-----BEGIN RSA PRIVATE KEY-----" => pemfile::rsa_private_keys,
        "-----BEGIN PRIVATE KEY-----" => pemfile::pkcs8_private_keys,
        _ => return Err(Error::new(Other, "invalid key header"))
    };

    let key = private_keys_fn(&mut Cursor::new(first_line).chain(reader))
        .map_err(|_| Error::new(Other, "invalid key file"))
        .and_then(|mut keys| match keys.len() {
            0 => Err(Error::new(Other, "no valid keys found; is the file malformed?")),
            1 => Ok(keys.remove(0)),
            n => Err(Error::new(Other, format!("expected 1 key, found {}", n))),
        })?;

    // Ensure we can use the key.
    rustls::sign::any_supported_type(&key)
        .map_err(|_| Error::new(Other, "key parsed but is unusable"))
        .map(|_| key)
}

pub struct TlsListener {
    listener: TcpListener,
    acceptor: TlsAcceptor,
    state: TlsListenerState,
}

enum TlsListenerState {
    Listening,
    Accepting(Accept<TcpStream>),
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
                TlsListenerState::Listening => {
                    match self.listener.poll_accept(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                        Poll::Ready(Ok((stream, _addr))) => {
                            let fut = self.acceptor.accept(stream);
                            self.state = TlsListenerState::Accepting(fut);
                        }
                    }
                }
                TlsListenerState::Accepting(ref mut fut) => {
                    match Pin::new(fut).poll(cx) {
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

pub async fn bind_tls<C: io::BufRead + Send, K: io::BufRead + Send>(
    address: SocketAddr,
    mut cert_chain: C,
    mut private_key: K,
) -> io::Result<TlsListener> {
    let cert_chain = load_certs(&mut cert_chain).map_err(|e| {
        let msg = format!("malformed TLS certificate chain: {}", e);
        io::Error::new(e.kind(), msg)
    })?;

    let key = load_private_key(&mut private_key).map_err(|e| {
        let msg = format!("malformed TLS private key: {}", e);
        io::Error::new(e.kind(), msg)
    })?;

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

#[cfg(test)]
mod test {
    use super::*;

    use std::io::Cursor;

    macro_rules! tls_example_key {
        ($k:expr) => {
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/tls/private/", $k))
        }
    }

    #[test]
    fn verify_load_private_keys_of_different_types() -> io::Result<()> {
        let rsa_sha256_key = tls_example_key!("rsa_sha256_key.pem");
        let ecdsa_nistp256_sha256_key = tls_example_key!("ecdsa_nistp256_sha256_key_pkcs8.pem");
        let ecdsa_nistp384_sha384_key = tls_example_key!("ecdsa_nistp384_sha384_key_pkcs8.pem");
        let ed2551_key = tls_example_key!("ed25519_key.pem");

        load_private_key(&mut Cursor::new(rsa_sha256_key))?;
        load_private_key(&mut Cursor::new(ecdsa_nistp256_sha256_key))?;
        load_private_key(&mut Cursor::new(ecdsa_nistp384_sha384_key))?;
        load_private_key(&mut Cursor::new(ed2551_key))?;

        Ok(())
    }

    #[test]
    fn verify_load_certs_of_different_types() -> io::Result<()> {
        let rsa_sha256_cert = tls_example_key!("rsa_sha256_cert.pem");
        let ecdsa_nistp256_sha256_cert = tls_example_key!("ecdsa_nistp256_sha256_cert.pem");
        let ecdsa_nistp384_sha384_cert = tls_example_key!("ecdsa_nistp384_sha384_cert.pem");
        let ed2551_cert = tls_example_key!("ed25519_cert.pem");

        load_certs(&mut Cursor::new(rsa_sha256_cert))?;
        load_certs(&mut Cursor::new(ecdsa_nistp256_sha256_cert))?;
        load_certs(&mut Cursor::new(ecdsa_nistp384_sha384_cert))?;
        load_certs(&mut Cursor::new(ed2551_cert))?;

        Ok(())
    }
}
