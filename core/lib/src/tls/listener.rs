use std::io;
use std::sync::Arc;

use futures::TryFutureExt;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_rustls::LazyConfigAcceptor;
use rustls::server::{Acceptor, ServerConfig};

use crate::{Ignite, Rocket};
use crate::listener::{Bind, Certificates, Connection, Endpoint, Listener};
use crate::tls::{TlsConfig, Result, Error};
use super::resolver::DynResolver;

#[doc(inline)]
pub use tokio_rustls::server::TlsStream;

/// A TLS listener over some listener interface L.
pub struct TlsListener<L> {
    listener: L,
    config: TlsConfig,
    default: Arc<ServerConfig>,
}

impl<L> TlsListener<L>
    where L: Listener<Accept = <L as Listener>::Connection>,
{
    pub async fn from(
        listener: L,
        mut config: TlsConfig,
        resolver: Option<DynResolver>
    ) -> Result<TlsListener<L>> {
        config.resolver = resolver;

        Ok(TlsListener {
            default: Arc::new(config.server_config().await?),
            listener,
            config,
        })
    }
}

impl<L: Bind> Bind for TlsListener<L>
    where L: Listener<Accept = <L as Listener>::Connection>
{
    type Error = Error;

    async fn bind(rocket: &Rocket<Ignite>) -> Result<Self, Self::Error> {
        let listener = L::bind(rocket).map_err(|e| Error::Bind(Box::new(e))).await?;
        let config: TlsConfig = rocket.figment().extract_inner("tls")?;
        Self::from(listener, config, DynResolver::extract(rocket)).await
    }

    fn bind_endpoint(rocket: &Rocket<Ignite>) -> Result<Endpoint, Self::Error> {
        let config: TlsConfig = rocket.figment().extract_inner("tls")?;
        L::bind_endpoint(rocket)
            .map(|e| e.with_tls(&config))
            .map_err(|e| Error::Bind(Box::new(e)))
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
        let acceptor = LazyConfigAcceptor::new(Acceptor::default(), conn);
        let handshake = acceptor.await?;
        let hello = handshake.client_hello();
        let config = match &self.config.resolver {
            Some(r) => r.resolve(hello).await.unwrap_or_else(|| self.default.clone()),
            None => self.default.clone(),
        };

        handshake.into_stream(config).await
    }

    fn endpoint(&self) -> io::Result<Endpoint> {
        Ok(self.listener.endpoint()?.with_tls(&self.config))
    }
}

impl<C: Connection> Connection for TlsStream<C> {
    fn endpoint(&self) -> io::Result<Endpoint> {
        Ok(self.get_ref().0.endpoint()?.assume_tls())
    }

    fn certificates(&self) -> Option<Certificates<'_>> {
        #[cfg(feature = "mtls")] {
            let cert_chain = self.get_ref().1.peer_certificates()?;
            Some(Certificates::from(cert_chain))
        }

        #[cfg(not(feature = "mtls"))]
        None
    }
}
