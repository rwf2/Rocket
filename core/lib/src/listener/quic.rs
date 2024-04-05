//! Experimental support for Quic and HTTP/3.
//!
//! To enable Rocket's experimental support for HTTP/3 and Quic, enable the
//! `http3-preview` feature and provide a valid TLS configuration:
//!
//! ```toml
//! // Add the following to your Cargo.toml:
//! [dependencies]
//! rocket = { version = "0.6.0-dev", features = ["http3-preview"] }
//!
//! // In your Rocket.toml or other equivalent config source:
//! [default.tls]
//! certs = "private/rsa_sha256_cert.pem"
//! key = "private/rsa_sha256_key.pem"
//! ```
//!
//! The launch message confirms that Rocket is serving traffic over Quic in
//! addition to TCP:
//!
//! ```sh
//! > 🚀 Rocket has launched on https://127.0.0.1:8000 (QUIC + mTLS)
//! ```
//!
//! mTLS is not yet supported via this implementation.

use std::io;
use std::fmt;
use std::net::SocketAddr;
use std::pin::pin;

use s2n_quic as quic;
use s2n_quic_h3 as quic_h3;
use quic_h3::h3 as h3;

use bytes::Bytes;
use futures::Stream;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

use crate::tls::{TlsConfig, Error};
use crate::listener::{Listener, Connection, Endpoint};

type H3Conn = h3::server::Connection<quic_h3::Connection, bytes::Bytes>;

pub struct QuicListener {
    endpoint: SocketAddr,
    listener: Mutex<quic::Server>,
    tls: TlsConfig,
}

pub struct H3Stream(H3Conn);

pub struct H3Connection {
    pub handle: quic::connection::Handle,
    pub parts: http::request::Parts,
    pub tx: QuicTx,
    pub rx: QuicRx,
}

pub struct QuicRx(h3::server::RequestStream<quic_h3::RecvStream, Bytes>);

pub struct QuicTx(h3::server::RequestStream<quic_h3::SendStream<Bytes>, Bytes>);

impl QuicListener {
    pub async fn bind(address: SocketAddr, tls: TlsConfig) -> Result<Self, Error> {
        use quic::provider::tls::rustls::Server as H3TlsServer;

        let cert_chain = tls.load_certs()?
            .into_iter()
            .map(|v| v.to_vec())
            .collect::<Vec<_>>();

        let h3tls = H3TlsServer::builder()
            .with_application_protocols(["h3"].into_iter())
            .map_err(|e| Error::Bind(e))?
            .with_certificate(cert_chain, tls.load_key()?.secret_der())
            .map_err(|e| Error::Bind(e))?
            .with_prefer_server_cipher_suite_order(tls.prefer_server_cipher_order)
            .map_err(|e| Error::Bind(e))?
            .build()
            .map_err(|e| Error::Bind(e))?;

        let listener = quic::Server::builder()
            .with_tls(h3tls)?
            .with_io(address)?
            .start()
            .map_err(|e| Error::Bind(Box::new(e)))?;

        Ok(QuicListener {
            tls,
            endpoint: listener.local_addr()?,
            listener: Mutex::new(listener),
        })
    }
}

impl Listener for QuicListener {
    type Accept = quic::Connection;

    type Connection = H3Stream;

    async fn accept(&self) -> io::Result<Self::Accept> {
        self.listener
            .lock().await
            .accept().await
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "closed"))
    }

    async fn connect(&self, accept: Self::Accept) -> io::Result<Self::Connection> {
        let quic_conn = quic_h3::Connection::new(accept);
        let conn = H3Conn::new(quic_conn).await.map_err(io::Error::other)?;
        Ok(H3Stream(conn))
    }

    fn endpoint(&self) -> io::Result<Endpoint> {
        Ok(Endpoint::Quic(self.endpoint).with_tls(&self.tls))
    }
}

impl H3Stream {
    pub async fn accept(&mut self) -> io::Result<Option<H3Connection>> {
        let handle = self.0.inner.conn.handle().clone();
        let ((parts, _), (tx, rx)) = match self.0.accept().await {
            Ok(Some((req, stream))) => (req.into_parts(), stream.split()),
            Ok(None) => return Ok(None),
            Err(e) => {
                if matches!(e.try_get_code().map(|c| c.value()), Some(0 | 0x100)) {
                    return Ok(None)
                }

                return Err(io::Error::other(e));
            }
        };

        Ok(Some(H3Connection { handle, parts, tx: QuicTx(tx), rx: QuicRx(rx) }))
    }
}

impl QuicTx {
    pub async fn send_response<S>(&mut self, response: http::Response<S>) -> io::Result<()>
        where S: Stream<Item = io::Result<Bytes>>
    {
        let (parts, body) = response.into_parts();
        let response = http::Response::from_parts(parts, ());
        self.0.send_response(response).await.map_err(io::Error::other)?;

        let mut body = pin!(body);
        while let Some(bytes) = body.next().await {
            let bytes = bytes.map_err(io::Error::other)?;
            self.0.send_data(bytes).await.map_err(io::Error::other)?;
        }

        self.0.finish().await.map_err(io::Error::other)
    }

    pub fn cancel(&mut self) {
        self.0.stop_stream(h3::error::Code::H3_NO_ERROR);
    }
}

// FIXME: Expose certificates when possible.
impl Connection for H3Stream {
    fn endpoint(&self) -> io::Result<Endpoint> {
        let addr = self.0.inner.conn.handle().remote_addr()?;
        Ok(Endpoint::Quic(addr).assume_tls())
    }
}

// FIXME: Expose certificates when possible.
impl Connection for H3Connection {
    fn endpoint(&self) -> io::Result<Endpoint> {
        let addr = self.handle.remote_addr()?;
        Ok(Endpoint::Quic(addr).assume_tls())
    }
}

mod async_traits {
    use std::io;
    use std::pin::Pin;
    use std::task::{ready, Context, Poll};

    use super::{Bytes, QuicRx};
    use crate::listener::AsyncCancel;

    use futures::Stream;
    use s2n_quic_h3::h3;

    impl Stream for QuicRx {
        type Item = io::Result<Bytes>;

        fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            use bytes::Buf;

            match ready!(self.0.poll_recv_data(cx)) {
                Ok(Some(mut buf)) => Poll::Ready(Some(Ok(buf.copy_to_bytes(buf.remaining())))),
                Ok(None) => Poll::Ready(None),
                Err(e) => Poll::Ready(Some(Err(io::Error::other(e)))),
            }
        }
    }

    impl AsyncCancel for QuicRx {
        fn poll_cancel(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
            self.0.stop_sending(h3::error::Code::H3_NO_ERROR);
            Poll::Ready(Ok(()))
        }
    }
}

impl fmt::Debug for H3Stream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("H3Stream").finish()
    }
}

impl fmt::Debug for H3Connection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("H3Connection").finish()
    }
}
