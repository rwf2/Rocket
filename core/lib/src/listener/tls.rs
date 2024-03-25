use std::io;
use std::sync::Arc;

use serde::Deserialize;
use rustls::server::{ServerSessionMemoryCache, ServerConfig, WebPkiClientVerifier};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_rustls::TlsAcceptor;

use crate::tls::{TlsConfig, Error};
use crate::tls::util::{self, load_cert_chain, load_key, load_ca_certs};
use crate::listener::{Listener, Bindable, Connection, Certificates, Endpoint};

#[doc(inline)]
pub use tokio_rustls::server::TlsStream;

/// A TLS listener over some listener interface L.
pub struct TlsListener<L> {
    listener: L,
    acceptor: TlsAcceptor,
    config: TlsConfig,
}

#[derive(Clone, Deserialize)]
pub struct TlsBindable<I> {
    #[serde(flatten)]
    pub inner: I,
    pub tls: TlsConfig,
}

impl TlsConfig {
    pub(crate) fn server_config(&self) -> Result<ServerConfig, Error> {
        let provider = rustls::crypto::CryptoProvider {
            cipher_suites: self.ciphers().map(|c| c.into()).collect(),
            ..util::get_crypto_provider()
        };

        #[cfg(feature = "mtls")]
        let verifier = match self.mutual {
            Some(ref mtls) => {
                let ca_certs = load_ca_certs(&mut mtls.ca_certs_reader()?)?;
                let verifier = WebPkiClientVerifier::builder(Arc::new(ca_certs));
                match mtls.mandatory {
                    true => verifier.build()?,
                    false => verifier.allow_unauthenticated().build()?,
                }
            },
            None => WebPkiClientVerifier::no_client_auth(),
        };

        #[cfg(not(feature = "mtls"))]
        let verifier = WebPkiClientVerifier::no_client_auth();

        let key = load_key(&mut self.key_reader()?)?;
        let cert_chain = load_cert_chain(&mut self.certs_reader()?)?;
        let mut tls_config = ServerConfig::builder_with_provider(Arc::new(provider))
            .with_safe_default_protocol_versions()?
            .with_client_cert_verifier(verifier)
            .with_single_cert(cert_chain, key)?;

        tls_config.ignore_client_order = self.prefer_server_cipher_order;
        tls_config.session_storage = ServerSessionMemoryCache::new(1024);
        tls_config.ticketer = rustls::crypto::ring::Ticketer::new()?;
        tls_config.alpn_protocols = vec![b"http/1.1".to_vec()];
        if cfg!(feature = "http2") {
            tls_config.alpn_protocols.insert(0, b"h2".to_vec());
        }

        Ok(tls_config)
    }
}

impl<I: Bindable> Bindable for TlsBindable<I>
    where I::Listener: Listener<Accept = <I::Listener as Listener>::Connection>,
          <I::Listener as Listener>::Connection: AsyncRead + AsyncWrite
{
    type Listener = TlsListener<I::Listener>;

    type Error = Error;

    async fn bind(self) -> Result<Self::Listener, Self::Error> {
        Ok(TlsListener {
            acceptor: TlsAcceptor::from(Arc::new(self.tls.server_config()?)),
            listener: self.inner.bind().await.map_err(|e| Error::Bind(Box::new(e)))?,
            config: self.tls,
        })
    }

    fn bind_endpoint(&self) -> io::Result<Endpoint> {
        let inner = self.inner.bind_endpoint()?;
        Ok(inner.with_tls(&self.tls))
    }
}

impl<L> Listener for TlsListener<L>
    where L: Listener<Accept = <L as Listener>::Connection>,
          L::Connection: AsyncRead + AsyncWrite
{
    type Accept = L::Connection;

    type Connection = TlsStream<L::Connection>;

    async fn accept(&self) -> io::Result<Self::Accept> {
        self.listener.accept().await
    }

    async fn connect(&self, conn: L::Connection) -> io::Result<Self::Connection> {
        self.acceptor.accept(conn).await
    }

    fn endpoint(&self) -> io::Result<Endpoint> {
        Ok(self.listener.endpoint()?.with_tls(&self.config))
    }
}

impl<C: Connection> Connection for TlsStream<C> {
    fn endpoint(&self) -> io::Result<Endpoint> {
        Ok(self.get_ref().0.endpoint()?.assume_tls())
    }

    #[cfg(feature = "mtls")]
    fn certificates(&self) -> Option<Certificates<'_>> {
        let cert_chain = self.get_ref().1.peer_certificates()?;
        Some(Certificates::from(cert_chain))
    }
}
