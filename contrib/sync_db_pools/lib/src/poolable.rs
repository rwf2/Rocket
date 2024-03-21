#[allow(unused)]
use std::time::Duration;

use r2d2::ManageConnection;
use rocket::{Rocket, Build};

#[allow(unused_imports)]
use crate::{Config, Error};

/// Trait implemented by `r2d2`-based database adapters.
///
/// # Provided Implementations
///
/// Implementations of `Poolable` are provided for the following types:
///
///   * `diesel::MysqlConnection`
///   * `diesel::PgConnection`
///   * `diesel::SqliteConnection`
///   * `postgres::Connection`
///   * `rusqlite::Connection`
///
/// # Implementation Guide
///
/// As an r2d2-compatible database (or other resource) adapter provider,
/// implementing `Poolable` in your own library will enable Rocket users to
/// consume your adapter with its built-in connection pooling support.
///
/// ## Example
///
/// Consider a library `foo` with the following types:
///
///   * `foo::ConnectionManager`, which implements [`r2d2::ManageConnection`]
///   * `foo::Connection`, the `Connection` associated type of
///     `foo::ConnectionManager`
///   * `foo::Error`, errors resulting from manager instantiation
///
/// In order for Rocket to generate the required code to automatically provision
/// a r2d2 connection pool into application state, the `Poolable` trait needs to
/// be implemented for the connection type. The following example implements
/// `Poolable` for `foo::Connection`:
///
/// ```rust
/// # mod foo {
/// #     use std::fmt;
/// #     use rocket_sync_db_pools::r2d2;
/// #     #[derive(Debug)] pub struct Error;
/// #     impl std::error::Error for Error {  }
/// #     impl fmt::Display for Error {
/// #         fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { Ok(()) }
/// #     }
/// #
/// #     pub struct Connection;
/// #     pub struct ConnectionManager;
/// #
/// #     type Result<T> = std::result::Result<T, Error>;
/// #
/// #     impl ConnectionManager {
/// #         pub fn new(url: &str) -> Result<Self> { Err(Error) }
/// #     }
/// #
/// #     impl self::r2d2::ManageConnection for ConnectionManager {
/// #          type Connection = Connection;
/// #          type Error = Error;
/// #          fn connect(&self) -> Result<Connection> { panic!() }
/// #          fn is_valid(&self, _: &mut Connection) -> Result<()> { panic!() }
/// #          fn has_broken(&self, _: &mut Connection) -> bool { panic!() }
/// #     }
/// # }
/// use std::time::Duration;
/// use rocket::{Rocket, Build};
/// use rocket_sync_db_pools::{r2d2, Error, Config, Poolable, PoolResult};
///
/// impl Poolable for foo::Connection {
///     type Manager = foo::ConnectionManager;
///     type Error = foo::Error;
///
///     fn pool(db_name: &str, rocket: &Rocket<Build>) -> PoolResult<Self> {
///         let config = Config::from(db_name, rocket)?;
///         let manager = foo::ConnectionManager::new(&config.url).map_err(Error::Custom)?;
///         Ok(r2d2::Pool::builder()
///             .max_size(config.pool_size)
///             .connection_timeout(Duration::from_secs(config.timeout as u64))
///             .build(manager)?)
///     }
/// }
/// ```
///
/// In this example, `ConnectionManager::new()` method returns a `foo::Error` on
/// failure. The [`Error`] enum consolidates this type, the `r2d2::Error` type
/// that can result from `r2d2::Pool::builder()`, and the
/// [`figment::Error`](rocket::figment::Error) type from
/// `database::Config::from()`.
///
/// In the event that a connection manager isn't fallible (as is the case with
/// Diesel's r2d2 connection manager, for instance), the associated error type
/// for the `Poolable` implementation should be `std::convert::Infallible`.
///
/// For more concrete example, consult Rocket's existing implementations of
/// [`Poolable`].
pub trait Poolable: Send + Sized + 'static {
    /// The associated connection manager for the given connection type.
    type Manager: ManageConnection<Connection=Self>;

    /// The associated error type in the event that constructing the connection
    /// manager and/or the connection pool fails.
    type Error: std::fmt::Debug;

    /// Creates an `r2d2` connection pool for `Manager::Connection`, returning
    /// the pool on success.
    fn pool(db_name: &str, rocket: &Rocket<Build>) -> PoolResult<Self>;
}

/// A type alias for the return type of [`Poolable::pool()`].
#[allow(type_alias_bounds)]
pub type PoolResult<P: Poolable> = Result<r2d2::Pool<P::Manager>, Error<P::Error>>;

#[cfg(feature = "diesel_sqlite_pool")]
impl Poolable for diesel::SqliteConnection {
    type Manager = diesel::r2d2::ConnectionManager<diesel::SqliteConnection>;
    type Error = std::convert::Infallible;

    fn pool(db_name: &str, rocket: &Rocket<Build>) -> PoolResult<Self> {
        use diesel::{SqliteConnection, connection::SimpleConnection};
        use diesel::r2d2::{CustomizeConnection, ConnectionManager, Error, Pool};

        #[derive(Debug)]
        struct Customizer;

        impl CustomizeConnection<SqliteConnection, Error> for Customizer {
            fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), Error> {
                conn.batch_execute("\
                    PRAGMA journal_mode = WAL;\
                    PRAGMA busy_timeout = 1000;\
                    PRAGMA foreign_keys = ON;\
                ").map_err(Error::QueryError)?;

                Ok(())
            }
        }

        let config = Config::from(db_name, rocket)?;
        let manager = ConnectionManager::new(&config.url);
        let pool = Pool::builder()
            .connection_customizer(Box::new(Customizer))
            .max_size(config.pool_size)
            .connection_timeout(Duration::from_secs(config.timeout as u64))
            .build(manager)?;

        Ok(pool)
    }
}

#[cfg(feature = "diesel_postgres_pool")]
impl Poolable for diesel::PgConnection {
    type Manager = diesel::r2d2::ConnectionManager<diesel::PgConnection>;
    type Error = std::convert::Infallible;

    fn pool(db_name: &str, rocket: &Rocket<Build>) -> PoolResult<Self> {
        let config = Config::from(db_name, rocket)?;
        let manager = diesel::r2d2::ConnectionManager::new(&config.url);
        let pool = r2d2::Pool::builder()
            .max_size(config.pool_size)
            .connection_timeout(Duration::from_secs(config.timeout as u64))
            .build(manager)?;

        Ok(pool)
    }
}

#[cfg(feature = "diesel_mysql_pool")]
impl Poolable for diesel::MysqlConnection {
    type Manager = diesel::r2d2::ConnectionManager<diesel::MysqlConnection>;
    type Error = std::convert::Infallible;

    fn pool(db_name: &str, rocket: &Rocket<Build>) -> PoolResult<Self> {
        let config = Config::from(db_name, rocket)?;
        let manager = diesel::r2d2::ConnectionManager::new(&config.url);
        let pool = r2d2::Pool::builder()
            .max_size(config.pool_size)
            .connection_timeout(Duration::from_secs(config.timeout as u64))
            .build(manager)?;

        Ok(pool)
    }
}

#[cfg(feature = "postgres_pool")]
pub mod pg {
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use std::io;

    #[derive(Clone)]
    pub enum MaybeTlsConnector {
        NoTls(postgres::tls::NoTls),
        #[cfg(feature = "postgres_pool_tls")]
        Tls(postgres_native_tls::MakeTlsConnector)
    }

    impl postgres::tls::MakeTlsConnect<postgres::Socket> for MaybeTlsConnector {
        type Stream = MaybeTlsConnector_Stream;
        type TlsConnect = MaybeTlsConnector_TlsConnect;
        type Error = MaybeTlsConnector_Error;

        fn make_tls_connect(&mut self, domain: &str) -> Result<Self::TlsConnect, Self::Error> {
            match self {
                MaybeTlsConnector::NoTls(connector) => {
                    <postgres::tls::NoTls as postgres::tls::MakeTlsConnect<postgres::Socket>>
                        ::make_tls_connect(connector, domain)
                        .map(Self::TlsConnect::NoTls)
                        .map_err(Self::Error::NoTls)
                },
                #[cfg(feature = "postgres_pool_tls")]
                MaybeTlsConnector::Tls(connector) => {
                    <
                        postgres_native_tls::MakeTlsConnector as
                        postgres::tls::MakeTlsConnect<postgres::Socket>
                    >::make_tls_connect(connector, domain)
                        .map(Self::TlsConnect::Tls)
                        .map_err(Self::Error::Tls)
                },
            }
        }
    }

    // --- Stream ---

    #[allow(non_camel_case_types)]
    pub enum MaybeTlsConnector_Stream {
        NoTls(postgres::tls::NoTlsStream),
        #[cfg(feature = "postgres_pool_tls")]
        Tls(postgres_native_tls::TlsStream<postgres::Socket>)
    }

    impl postgres::tls::TlsStream for MaybeTlsConnector_Stream {
        fn channel_binding(&self) -> postgres::tls::ChannelBinding {
            match self {
                MaybeTlsConnector_Stream::NoTls(stream) => stream.channel_binding(),
                #[cfg(feature = "postgres_pool_tls")]
                MaybeTlsConnector_Stream::Tls(stream) => stream.channel_binding(),
            }
        }
    }

    impl tokio::io::AsyncRead for MaybeTlsConnector_Stream {
        fn poll_read(
                mut self: Pin<&mut Self>,
                cx: &mut Context<'_>,
                buf: &mut tokio::io::ReadBuf<'_>
            ) -> Poll<Result<(), io::Error>> {
            match *self {
                MaybeTlsConnector_Stream::NoTls(ref mut stream) =>
                    Pin::new(stream).poll_read(cx, buf),
                #[cfg(feature = "postgres_pool_tls")]
                MaybeTlsConnector_Stream::Tls(ref mut stream) =>
                    Pin::new(stream).poll_read(cx, buf),
            }
        }
    }

    impl tokio::io::AsyncWrite for MaybeTlsConnector_Stream {
        fn poll_write(
                mut self: Pin<&mut Self>,
                cx: &mut Context<'_>,
                buf: &[u8]
            ) -> Poll<io::Result<usize>> {
            match *self {
                MaybeTlsConnector_Stream::NoTls(ref mut stream) =>
                    Pin::new(stream).poll_write(cx, buf),
                #[cfg(feature = "postgres_pool_tls")]
                MaybeTlsConnector_Stream::Tls(ref mut stream) =>
                    Pin::new(stream).poll_write(cx, buf),
            }
        }

        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            match *self {
                MaybeTlsConnector_Stream::NoTls(ref mut stream) => Pin::new(stream).poll_flush(cx),
                #[cfg(feature = "postgres_pool_tls")]
                MaybeTlsConnector_Stream::Tls(ref mut stream) => Pin::new(stream).poll_flush(cx),
            }
        }

        fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            match *self {
                MaybeTlsConnector_Stream::NoTls(ref mut stream) =>
                    Pin::new(stream).poll_shutdown(cx),
                #[cfg(feature = "postgres_pool_tls")]
                MaybeTlsConnector_Stream::Tls(ref mut stream) =>
                    Pin::new(stream).poll_shutdown(cx),
            }
        }
    }

    // --- TlsConnect ---

    #[allow(non_camel_case_types)]
    pub enum MaybeTlsConnector_TlsConnect {
        NoTls(postgres::tls::NoTls),
        #[cfg(feature = "postgres_pool_tls")]
        Tls(postgres_native_tls::TlsConnector)
    }

    impl postgres::tls::TlsConnect<postgres::Socket> for MaybeTlsConnector_TlsConnect {
        type Stream = MaybeTlsConnector_Stream;
        type Error = MaybeTlsConnector_Error;
        type Future = MaybeTlsConnector_Future;

        fn connect(self, socket: postgres::Socket) -> Self::Future {
            match self {
                MaybeTlsConnector_TlsConnect::NoTls(connector) =>
                    Self::Future::NoTls(connector.connect(socket)),
                #[cfg(feature = "postgres_pool_tls")]
                MaybeTlsConnector_TlsConnect::Tls(connector) =>
                    Self::Future::Tls(connector.connect(socket)),
            }
        }
    }

    // --- Error ---

    #[allow(non_camel_case_types)]
    #[derive(Debug)]
    pub enum MaybeTlsConnector_Error {
        NoTls(postgres::tls::NoTlsError),
        #[cfg(feature = "postgres_pool_tls")]
        Tls(native_tls::Error)
    }

    impl std::fmt::Display for MaybeTlsConnector_Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                MaybeTlsConnector_Error::NoTls(e) => e.fmt(f),
                #[cfg(feature = "postgres_pool_tls")]
                MaybeTlsConnector_Error::Tls(e) => e.fmt(f),
            }
        }
    }

    impl std::error::Error for MaybeTlsConnector_Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                MaybeTlsConnector_Error::NoTls(e) => e.source(),
                #[cfg(feature = "postgres_pool_tls")]
                MaybeTlsConnector_Error::Tls(e) => e.source(),
            }
        }
    }

    // --- Future ---

    #[allow(non_camel_case_types)]
    pub enum MaybeTlsConnector_Future {
        NoTls(postgres::tls::NoTlsFuture),
        #[cfg(feature = "postgres_pool_tls")]
        Tls(<postgres_native_tls::TlsConnector as
            postgres::tls::TlsConnect<postgres::Socket>>::Future)
    }

    impl std::future::Future for MaybeTlsConnector_Future {
        type Output = Result<MaybeTlsConnector_Stream, MaybeTlsConnector_Error>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            match *self {
                MaybeTlsConnector_Future::NoTls(ref mut future) => {
                    Pin::new(future)
                        .poll(cx)
                        .map(|v| v.map(MaybeTlsConnector_Stream::NoTls))
                        .map_err(MaybeTlsConnector_Error::NoTls)
                },
                #[cfg(feature = "postgres_pool_tls")]
                MaybeTlsConnector_Future::Tls(ref mut future) => {
                    Pin::new(future)
                        .poll(cx)
                        .map(|v| v.map(MaybeTlsConnector_Stream::Tls))
                        .map_err(MaybeTlsConnector_Error::Tls)
                }
            }
        }
    }
}

#[cfg(feature = "postgres_pool")]
impl Poolable for postgres::Client {
    type Manager = r2d2_postgres::PostgresConnectionManager<pg::MaybeTlsConnector>;
    type Error = postgres::Error;

    fn pool(db_name: &str, rocket: &Rocket<Build>) -> PoolResult<Self> {
        let config = Config::from(db_name, rocket)?;
        let url = config.url.parse().map_err(Error::Custom)?;

        let tls_connector = match config.tls {
            // `tls_config` is unused when `postgres_pool_tls` is disabled.
            #[allow(unused_variables)]
            Some(ref tls_config) => {

                #[cfg(feature = "postgres_pool_tls")]
                {
                    let mut connector_builder = native_tls::TlsConnector::builder();
                    if let Some(ref cert) = tls_config.ssl_root_cert {
                        let cert_file_bytes = std::fs::read(cert)?;
                        let cert = native_tls::Certificate::from_pem(&cert_file_bytes)
                            .map_err(|e| Error::Tls(e.into()))?;
                        connector_builder.add_root_certificate(cert);

                        // Client certs
                        match (
                            tls_config.ssl_client_cert.as_ref(),
                            tls_config.ssl_client_key.as_ref(),
                        ) {
                            (Some(cert), Some(key)) => {
                                let cert_file_bytes = std::fs::read(cert)?;
                                let key_file_bytes = std::fs::read(key)?;
                                let cert = native_tls::Identity::from_pkcs8(
                                        &cert_file_bytes,
                                        &key_file_bytes
                                    ).map_err(|e| Error::Tls(e.into()))?;
                                connector_builder.identity(cert);
                            },
                            (Some(_), None) => {
                                return Err(Error::Tls(
                                    "Client certificate provided without client key".into()
                                ))
                            },
                            (None, Some(_)) => {
                                return Err(Error::Tls(
                                    "Client key provided without client certificate".into()
                                ))
                            },
                            (None, None) => {},
                        }
                    }

                    connector_builder
                        .danger_accept_invalid_certs(tls_config.accept_invalid_certs);
                    connector_builder
                        .danger_accept_invalid_hostnames(tls_config.accept_invalid_hostnames);

                    pg::MaybeTlsConnector::Tls(postgres_native_tls::MakeTlsConnector::new(
                        connector_builder.build().map_err(|e| Error::Tls(e.into()))?
                    ))
                }

                #[cfg(not(feature = "postgres_pool_tls"))]
                {
                    // TODO: Should this be an error?
                    rocket::warn!("The `postgres_pool_tls` feature is disabled. \
                        Postgres TLS configuration will be ignored.");
                    pg::MaybeTlsConnector::NoTls(postgres::tls::NoTls)
                }
            },
            None => {
                pg::MaybeTlsConnector::NoTls(postgres::tls::NoTls)
            }
        };

        let manager = r2d2_postgres::PostgresConnectionManager::new(url, tls_connector);
        let pool = r2d2::Pool::builder()
            .max_size(config.pool_size)
            .connection_timeout(Duration::from_secs(config.timeout as u64))
            .build(manager)?;

        Ok(pool)
    }
}

#[cfg(feature = "sqlite_pool")]
impl Poolable for rusqlite::Connection {
    type Manager = r2d2_sqlite::SqliteConnectionManager;
    type Error = std::convert::Infallible;

    fn pool(db_name: &str, rocket: &Rocket<Build>) -> PoolResult<Self> {
        use rocket::figment::providers::Serialized;

        #[derive(Debug, serde::Deserialize, serde::Serialize)]
        #[serde(rename_all = "snake_case")]
        enum OpenFlag {
            ReadOnly,
            ReadWrite,
            Create,
            Uri,
            Memory,
            NoMutex,
            FullMutex,
            SharedCache,
            PrivateCache,
            Nofollow,
        }

        let figment = Config::figment(db_name, rocket);
        let config: Config = figment.extract()?;
        let open_flags: Vec<OpenFlag> = figment
            .join(Serialized::default("open_flags", <Vec<OpenFlag>>::new()))
            .extract_inner("open_flags")?;

        let mut flags = rusqlite::OpenFlags::default();
        for flag in open_flags {
            let sql_flag = match flag {
                OpenFlag::ReadOnly => rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
                OpenFlag::ReadWrite => rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE,
                OpenFlag::Create => rusqlite::OpenFlags::SQLITE_OPEN_CREATE,
                OpenFlag::Uri => rusqlite::OpenFlags::SQLITE_OPEN_URI,
                OpenFlag::Memory => rusqlite::OpenFlags::SQLITE_OPEN_MEMORY,
                OpenFlag::NoMutex => rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
                OpenFlag::FullMutex => rusqlite::OpenFlags::SQLITE_OPEN_FULL_MUTEX,
                OpenFlag::SharedCache => rusqlite::OpenFlags::SQLITE_OPEN_SHARED_CACHE,
                OpenFlag::PrivateCache => rusqlite::OpenFlags::SQLITE_OPEN_PRIVATE_CACHE,
                OpenFlag::Nofollow => rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW,
            };

            flags.insert(sql_flag)
        };

        let manager = r2d2_sqlite::SqliteConnectionManager::file(&*config.url)
            .with_flags(flags);

        let pool = r2d2::Pool::builder()
            .max_size(config.pool_size)
            .connection_timeout(Duration::from_secs(config.timeout as u64))
            .build(manager)?;

        Ok(pool)
    }
}

#[cfg(feature = "memcache_pool")]
impl Poolable for memcache::Client {
    type Manager = r2d2_memcache::MemcacheConnectionManager;
    // Unused, but we might want it in the future without a breaking change.
    type Error = memcache::MemcacheError;

    fn pool(db_name: &str, rocket: &Rocket<Build>) -> PoolResult<Self> {
        let config = Config::from(db_name, rocket)?;
        let manager = r2d2_memcache::MemcacheConnectionManager::new(&*config.url);
        let pool = r2d2::Pool::builder()
            .max_size(config.pool_size)
            .connection_timeout(Duration::from_secs(config.timeout as u64))
            .build(manager)?;

        Ok(pool)
    }
}
