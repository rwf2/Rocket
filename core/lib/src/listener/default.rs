use core::fmt;

use either::Either;
use serde::Deserialize;
use tokio_util::either::Either::{Left, Right};

use crate::listener::{tcp::TcpListener, Bind, Endpoint};
use crate::{Ignite, Rocket};

#[cfg(unix)]
use crate::listener::unix::UnixListener;
#[cfg(feature = "tls")]
use crate::tls::{TlsConfig, TlsListener};

mod private {
    use super::*;
    use tokio_util::either::Either;

    #[cfg(feature = "tls")]
    type TlsListener<T> = super::TlsListener<T>;
    #[cfg(not(feature = "tls"))]
    type TlsListener<T> = T;
    #[cfg(unix)]
    type UnixListener = super::UnixListener;
    #[cfg(not(unix))]
    type UnixListener = TcpListener;

    pub type Listener = Either<
        Either<TlsListener<TcpListener>, TlsListener<UnixListener>>,
        Either<TcpListener, UnixListener>,
    >;

    /// The default connection listener.
    ///
    /// # Configuration
    ///
    /// Reads the following optional configuration parameters:
    ///
    /// | parameter   | type              | default               |
    /// | ----------- | ----------------- | --------------------- |
    /// | `address`   | [`Endpoint`]      | `tcp:127.0.0.1:8000`  |
    /// | `tls`       | [`TlsConfig`]     | None                  |
    /// | `reuse`     | boolean           | `true`                |
    ///
    /// # Listener
    ///
    /// Based on the above configuration, this listener defers to one of the
    /// following existing listeners:
    ///
    /// | listener                      | `address` type     | `tls` enabled |
    /// |-------------------------------|--------------------|---------------|
    /// | [`TcpListener`]               | [`Endpoint::Tcp`]  | no            |
    /// | [`UnixListener`]              | [`Endpoint::Unix`] | no            |
    /// | [`TlsListener<TcpListener>`]  | [`Endpoint::Tcp`]  | yes           |
    /// | [`TlsListener<UnixListener>`] | [`Endpoint::Unix`] | yes           |
    ///
    /// [`UnixListener`]: crate::listener::unix::UnixListener
    /// [`TlsListener<TcpListener>`]: crate::tls::TlsListener
    /// [`TlsListener<UnixListener>`]: crate::tls::TlsListener
    ///
    ///  * **address type** is the variant the `address` parameter parses as.
    ///  * **`tls` enabled** is `yes` when the `tls` feature is enabled _and_ a
    ///    `tls` configuration is provided.
    #[cfg(doc)]
    pub struct DefaultListener(());
}

#[derive(Deserialize)]
struct Config {
    #[serde(default)]
    address: Endpoint,
    #[cfg(feature = "tls")]
    tls: Option<TlsConfig>,
}

#[cfg(doc)]
pub use private::DefaultListener;

#[cfg(doc)]
type Connection = crate::listener::tcp::TcpStream;

#[cfg(doc)]
impl Bind for DefaultListener {
    type Error = Error;
    async fn bind(_: &Rocket<Ignite>) -> Result<Self, Error> {
        unreachable!()
    }
    fn bind_endpoint(_: &Rocket<Ignite>) -> Result<Endpoint, Error> {
        unreachable!()
    }
}

#[cfg(doc)]
impl super::Listener for DefaultListener {
    #[doc(hidden)]
    type Accept = Connection;
    #[doc(hidden)]
    type Connection = Connection;
    #[doc(hidden)]
    async fn accept(&self) -> std::io::Result<Connection> {
        unreachable!()
    }
    #[doc(hidden)]
    async fn connect(&self, _: Self::Accept) -> std::io::Result<Connection> {
        unreachable!()
    }
    #[doc(hidden)]
    fn endpoint(&self) -> std::io::Result<Endpoint> {
        unreachable!()
    }
}

#[cfg(not(doc))]
pub type DefaultListener = private::Listener;

#[cfg(not(doc))]
impl Bind for DefaultListener {
    type Error = Error;

    async fn bind(rocket: &Rocket<Ignite>) -> Result<Self, Self::Error> {
        let config: Config = rocket.figment().extract()?;
        match config.address {
            #[cfg(feature = "tls")]
            Endpoint::Tcp(_) if config.tls.is_some() => {
                let listener = <TlsListener<TcpListener> as Bind>::bind(rocket).await?;
                Ok(Left(Left(listener)))
            }
            Endpoint::Tcp(_) => {
                let listener = <TcpListener as Bind>::bind(rocket).await?;
                Ok(Right(Left(listener)))
            }
            #[cfg(all(unix, feature = "tls"))]
            Endpoint::Unix(_) if config.tls.is_some() => {
                let listener = <TlsListener<UnixListener> as Bind>::bind(rocket).await?;
                Ok(Left(Right(listener)))
            }
            #[cfg(unix)]
            Endpoint::Unix(_) => {
                let listener = <UnixListener as Bind>::bind(rocket).await?;
                Ok(Right(Right(listener)))
            }
            endpoint => Err(Error::Unsupported(endpoint)),
        }
    }

    fn bind_endpoint(rocket: &Rocket<Ignite>) -> Result<Endpoint, Self::Error> {
        let config: Config = rocket.figment().extract()?;
        Ok(config.address)
    }
}

#[derive(Debug)]
pub enum Error {
    Config(figment::Error),
    Io(std::io::Error),
    Unsupported(Endpoint),
    #[cfg(feature = "tls")]
    Tls(crate::tls::Error),
}

impl From<figment::Error> for Error {
    fn from(value: figment::Error) -> Self {
        Error::Config(value)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::Io(value)
    }
}

#[cfg(feature = "tls")]
impl From<crate::tls::Error> for Error {
    fn from(value: crate::tls::Error) -> Self {
        Error::Tls(value)
    }
}

impl From<Either<figment::Error, std::io::Error>> for Error {
    fn from(value: Either<figment::Error, std::io::Error>) -> Self {
        value.either(Error::Config, Error::Io)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Config(e) => e.fmt(f),
            Error::Io(e) => e.fmt(f),
            Error::Unsupported(e) => write!(f, "unsupported endpoint: {e:?}"),
            #[cfg(feature = "tls")]
            Error::Tls(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Config(e) => Some(e),
            Error::Io(e) => Some(e),
            Error::Unsupported(_) => None,
            #[cfg(feature = "tls")]
            Error::Tls(e) => Some(e),
        }
    }
}
