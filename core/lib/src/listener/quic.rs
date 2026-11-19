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

use std::io;
use std::net::SocketAddr;
use std::pin::{pin, Pin};
use std::sync::Arc;
use std::task::{ready, Context, Poll};

use bytes::{Buf, Bytes};
use futures::{Stream, StreamExt};
use h3::server::RequestStream;
use quinn::crypto::rustls::QuicServerConfig;

use crate::tls::{TlsConfig, Error};
use crate::listener::{AsyncCancel, Certificates};
use crate::shutdown::Shutdown;

pub(crate) type Connection = h3::server::Connection<h3_quinn::Connection, Bytes>;

/// The receiving half of an HTTP/3 request stream: the request body.
pub struct QuicRx(pub(crate) RequestStream<h3_quinn::RecvStream, Bytes>);

/// The sending half of an HTTP/3 request stream: the response.
///
/// Dropping a `QuicTx` before the response is complete resets the stream so
/// that the client cannot mistake a truncated response for a complete one.
pub(crate) struct QuicTx {
    stream: RequestStream<h3_quinn::SendStream<Bytes>, Bytes>,
    grace: Shutdown,
    done: bool,
}

/// Binds a QUIC endpoint to `addr` using Rocket's TLS configuration `tls`.
pub(crate) async fn bind(addr: SocketAddr, tls: &TlsConfig) -> Result<quinn::Endpoint, Error> {
    let mut tls_config = tls.server_config().await?;
    tls_config.alpn_protocols = vec![b"h3".to_vec()];
    tls_config.max_early_data_size = u32::MAX;

    let quic_config = QuicServerConfig::try_from(tls_config)
        .map_err(|e| Error::Bind(Box::new(e)))?;

    let server_config = quinn::ServerConfig::with_crypto(Arc::new(quic_config));
    Ok(quinn::Endpoint::server(server_config, addr)?)
}

/// Returns the TLS certificate chain presented by the peer of `conn`, if any.
pub(crate) fn certificates(conn: &quinn::Connection) -> Option<Certificates<'static>> {
    #[cfg(feature = "mtls")] {
        let certs = conn.peer_identity()?;
        let certs = certs.downcast::<Vec<rustls::pki_types::CertificateDer<'static>>>().ok()?;
        Some(Certificates::from(*certs))
    }

    #[cfg(not(feature = "mtls"))] {
        let _ = conn;
        None
    }
}

/// Returns the server name (SNI) requested by the peer of `conn`, if any.
pub(crate) fn server_name(conn: &quinn::Connection) -> Option<String> {
    let data = conn.handshake_data()?;
    data.downcast::<quinn::crypto::rustls::HandshakeData>().ok()?.server_name
}

/// Sends `response`, streaming the body as it becomes available.
pub(crate) async fn send_response<S>(tx: &mut QuicTx, response: http::Response<S>) -> io::Result<()>
    where S: Stream<Item = io::Result<Bytes>>
{
    let (mut parts, body) = response.into_parts();
    strip_connection_headers(&mut parts.headers);
    let stream = &mut tx.stream;
    stream.send_response(http::Response::from_parts(parts, ())).await.map_err(io::Error::other)?;

    let mut body = pin!(body);
    while let Some(chunk) = body.next().await {
        stream.send_data(chunk?).await.map_err(io::Error::other)?;
    }

    stream.finish().await.map_err(io::Error::other)?;
    tx.done = true;
    Ok(())
}

/// Removes the connection-specific headers that HTTP/3 forbids (RFC 9114
/// §4.2): the same headers hyper removes from HTTP/2 responses.
fn strip_connection_headers(headers: &mut http::HeaderMap) {
    use http::header::{HeaderName, CONNECTION, TE, TRANSFER_ENCODING, UPGRADE};

    let keep_alive = HeaderName::from_static("keep-alive");
    let proxy_connection = HeaderName::from_static("proxy-connection");
    for name in [keep_alive, proxy_connection, TRANSFER_ENCODING, UPGRADE, TE] {
        headers.remove(name);
    }

    // `Connection` may name other headers that are specific to the connection.
    if let Some(names) = headers.remove(CONNECTION) {
        for name in names.to_str().unwrap_or_default().split(',') {
            headers.remove(name.trim());
        }
    }
}

impl QuicTx {
    /// Wraps the sending half of a request stream. A response that is cut
    /// short once `grace` has elapsed is reset as cancelled.
    pub(crate) fn new(
        stream: RequestStream<h3_quinn::SendStream<Bytes>, Bytes>,
        grace: Shutdown,
    ) -> Self {
        QuicTx { stream, grace, done: false }
    }
}

impl Drop for QuicTx {
    fn drop(&mut self) {
        if !self.done {
            let code = match self.grace.notified() {
                true => h3::error::Code::H3_REQUEST_CANCELLED,
                false => h3::error::Code::H3_INTERNAL_ERROR,
            };

            self.stream.stop_stream(code);
        }
    }
}

/// Logs an error from serving an HTTP/3 connection or request. A client
/// closing its connection, cancelling a request, or going idle is routine and
/// logged at `debug`. Other errors from h3 or quinn are logged at `warn`, as
/// hyper's are. Any other error is a server error.
pub(crate) fn log_error(error: &io::Error) {
    use h3::error::{Code, ConnectionError, LocalError, StreamError};
    use h3::quic::ConnectionErrorIncoming;

    fn quinn_routine(error: &quinn::ConnectionError) -> bool {
        use quinn::ConnectionError::*;

        match error {
            ApplicationClosed(close) => close.error_code.into_inner() == Code::H3_NO_ERROR.value(),
            Reset | TimedOut | LocallyClosed => true,
            _ => false,
        }
    }

    // h3's error variants are non-exhaustive, so fields are matched by name.
    fn h3_routine(error: &ConnectionError) -> bool {
        match error {
            ConnectionError::Timeout { .. } => true,
            ConnectionError::Local { error: LocalError::Closing { .. }, .. } => true,
            ConnectionError::Remote { 0: ConnectionErrorIncoming::Timeout, .. } => true,
            ConnectionError::Remote { 0: ConnectionErrorIncoming::Undefined(e), .. } => {
                e.downcast_ref().is_some_and(quinn_routine)
            }
            error => error.is_h3_no_error(),
        }
    }

    let Some(inner) = error.get_ref() else {
        return crate::error::log_server_error(error);
    };

    let routine = if let Some(e) = inner.downcast_ref::<ConnectionError>() {
        h3_routine(e)
    } else if let Some(e) = inner.downcast_ref::<StreamError>() {
        match e {
            StreamError::RemoteTerminate { .. } | StreamError::RemoteClosing { .. } => true,
            StreamError::ConnectionError { 0: e, .. } => h3_routine(e),
            _ => false,
        }
    } else if let Some(e) = inner.downcast_ref::<quinn::ConnectionError>() {
        quinn_routine(e)
    } else if inner.is::<tokio::time::error::Elapsed>() {
        false
    } else {
        return crate::error::log_server_error(error);
    };

    match routine {
        true => debug!(error = %error, "http/3 connection or request closed"),
        false => warn!(error = %error, "http/3 connection or request failed"),
    }
}

impl Stream for QuicRx {
    type Item = io::Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
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
