use rocket::figment::Figment;

#[allow(unused_imports)]
use {std::time::Duration, crate::{Error, Config}};

/// Generic [`Database`](crate::Database) driver connection pool trait.
///
/// This trait provides a generic interface to various database pooling
/// implementations in the Rust ecosystem. It can be implemented by anyone, but
/// this crate provides implementations for common drivers.
///
/// **Implementations of this trait outside of this crate should be rare. You
/// _do not_ need to implement this trait or understand its specifics to use
/// this crate.**
///
/// ## Async Trait
///
/// [`Pool`] is an _async_ trait. Implementations of `Pool` must be decorated
/// with an attribute of `#[async_trait]`:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// use rocket::figment::Figment;
/// use rocket_db_pools::Pool;
///
/// # struct MyPool;
/// # type Connection = ();
/// # type Error = std::convert::Infallible;
/// #[rocket::async_trait]
/// impl Pool for MyPool {
///     type Connection = Connection;
///
///     type Error = Error;
///
///     async fn init(figment: &Figment) -> Result<Self, Self::Error> {
///         todo!("initialize and return an instance of the pool");
///     }
///
///     async fn get(&self) -> Result<Self::Connection, Self::Error> {
///         todo!("fetch one connection from the pool");
///     }
///
///     async fn close(&self) {
///         todo!("gracefully shutdown connection pool");
///     }
/// }
/// ```
///
/// ## Implementing
///
/// Implementations of `Pool` typically trace the following outline:
///
///   1. The `Error` associated type is set to [`Error`].
///
///   2. A [`Config`] is [extracted](Figment::extract()) from the `figment`
///      passed to init.
///
///   3. The pool is initialized and returned in `init()`, wrapping
///      initialization errors in [`Error::Init`].
///
///   4. A connection is retrieved in `get()`, wrapping errors in
///      [`Error::Get`].
///
/// Concretely, this looks like:
///
/// ```rust
/// use rocket::figment::Figment;
/// use rocket_db_pools::{Pool, Config, Error};
/// #
/// # type InitError = std::convert::Infallible;
/// # type GetError = std::convert::Infallible;
/// # type Connection = ();
/// #
/// # struct MyPool(Config);
/// # impl MyPool {
/// #    fn new(c: Config) -> Result<Self, InitError> {
/// #        Ok(Self(c))
/// #    }
/// #
/// #    fn acquire(&self) -> Result<Connection, GetError> {
/// #        Ok(())
/// #    }
/// #
/// #   async fn shutdown(&self) { }
/// # }
///
/// #[rocket::async_trait]
/// impl Pool for MyPool {
///     type Connection = Connection;
///
///     type Error = Error<InitError, GetError>;
///
///     async fn init(figment: &Figment) -> Result<Self, Self::Error> {
///         // Extract the config from `figment`.
///         let config: Config = figment.extract()?;
///
///         // Read config values, initialize `MyPool`. Map errors of type
///         // `InitError` to `Error<InitError, _>` with `Error::Init`.
///         let pool = MyPool::new(config).map_err(Error::Init)?;
///
///         // Return the fully initialized pool.
///         Ok(pool)
///     }
///
///     async fn get(&self) -> Result<Self::Connection, Self::Error> {
///         // Get one connection from the pool, here via an `acquire()` method.
///         // Map errors of type `GetError` to `Error<_, GetError>`.
///         self.acquire().map_err(Error::Get)
///     }
///
///     async fn close(&self) {
///         self.shutdown().await;
///     }
/// }
/// ```
#[rocket::async_trait]
pub trait Pool: Sized + Send + Sync + 'static {
    /// The connection type managed by this pool, returned by [`Self::get()`].
    type Connection;

    /// The error type returned by [`Self::init()`] and [`Self::get()`].
    type Error: std::error::Error;

    /// Constructs a pool from the configuration in `figment`.
    ///
    /// It is up to each implementor of `Pool` to define its accepted
    /// configuration values. Integrations provided in `rocket_db_pools` use
    /// [`Config`], which requires a `url` and accepts optional pool settings
    /// such as `max_connections` and `connect_timeout`.
    ///
    /// ## Errors
    ///
    /// This method returns an error if the configuration is not compatible, or
    /// if creating a pool failed due to an unavailable database server,
    /// insufficient resources, or another database-specific error.
    async fn init(figment: &Figment) -> Result<Self, Self::Error>;

    /// Asynchronously retrieves a connection from the factory or pool.
    ///
    /// ## Errors
    ///
    /// This method returns an error if a connection could not be retrieved,
    /// such as a preconfigured timeout elapsing or when the database server is
    /// unavailable.
    async fn get(&self) -> Result<Self::Connection, Self::Error>;

    /// Shutdown the connection pool, disallowing any new connections from being
    /// retrieved and waking up any tasks with active connections.
    ///
    /// The returned future may either resolve when all connections are known to
    /// have closed or at any point prior. Details are implementation specific.
    async fn close(&self);
}

/// A [`Pool`] that supports customization during initialization.
///
/// This trait enables [`Database::custom()`](crate::Database::custom()) and
/// [`Database::try_custom()`](crate::Database::try_custom()). It is implemented
/// for SQLx pools and MongoDB clients.
#[rocket::async_trait]
pub trait Configurable: Pool {
    /// The driver's configuration options.
    ///
    /// The configuration types for the supported drivers are:
    ///
    ///   * SQLx: `(sqlx::pool::PoolOptions<D>, ConnectOptions)`, where `D` is
    ///     `Sqlite`, `Postgres`, or `MySql`, and `ConnectOptions` is
    ///     `SqliteConnectOptions`, `PgConnectOptions`, or
    ///     `MySqlConnectOptions`, respectively.
    ///   * MongoDB: `mongodb::options::ClientOptions`.
    type Options: Send;

    /// Constructs a pool from `figment`, customizing its options with `f`.
    ///
    /// Implementations should extract [`Config`] from `figment` and apply its
    /// values and the driver's defaults to [`Self::Options`]. The function `f`
    /// is then called with the configuration and options. The returned options
    /// are used to create the pool.
    ///
    /// Implementations should share this initialization path with
    /// [`Pool::init()`]. Any error returned by `f` is returned to the caller.
    async fn init_with<F>(figment: &Figment, f: F) -> Result<Self, Self::Error>
        where F: FnOnce(&Config, Self::Options) -> Result<Self::Options, Self::Error> + Send;
}

/// Returns a TLS connector for `tokio-postgres` that verifies servers against
/// the platform's native root certificates, or those in `SSL_CERT_FILE` and
/// `SSL_CERT_DIR` when set.
///
/// Failing to find root certificates is not an error here: connections that
/// don't use TLS are unaffected, and those that do fail to verify the server.
#[cfg(all(
    feature = "postgres_rustls_native_certs",
    any(feature = "deadpool_postgres", feature = "diesel_postgres")
))]
fn tokio_postgres_tls_provider() -> tokio_postgres_rustls::MakeRustlsConnect {
    use std::sync::Arc;

    let native = rustls_native_certs::load_native_certs();
    for e in &native.errors {
        rocket::warn!("Failed to load native root certificates: {e}");
    }

    let mut roots = rustls::RootCertStore::empty();
    let (added, ignored) = roots.add_parsable_certificates(native.certs);
    if ignored > 0 {
        rocket::warn!("Ignored {ignored} unparsable native root certificate(s).");
    }

    if added == 0 {
        rocket::warn!("No root certificates found: Postgres TLS connections will fail.");
        rocket::info_!("Set SSL_CERT_FILE or SSL_CERT_DIR to trust a custom CA bundle.");
    }

    // Use `ring` directly: no `CryptoProvider` is installed by default.
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .expect("ring's default provider supports the default protocol versions")
        .with_root_certificates(roots)
        .with_no_client_auth();

    tokio_postgres_rustls::MakeRustlsConnect::new(config)
}

#[cfg(feature = "deadpool")]
mod deadpool {
    use deadpool::{Runtime, managed::{Manager, Pool, PoolError, Object}};
    use super::{Duration, Error, Config, Figment};

    #[cfg(any(feature = "diesel_postgres", feature = "diesel_mysql"))]
    use diesel_async::pooled_connection::AsyncDieselConnectionManager;

    pub trait DeadManager: Manager + Sized + Send + Sync + 'static {
        fn new(config: &Config, figment: &Figment) -> Result<Self, Error<PoolError<Self::Error>>>;
    }

    #[cfg(feature = "deadpool_postgres")]
    impl DeadManager for deadpool_postgres::Manager {
        fn new(config: &Config, _figment: &Figment) -> Result<Self, Error<PoolError<Self::Error>>> {
            #[cfg(feature = "postgres_rustls_native_certs")]
            let tls_provider = super::tokio_postgres_tls_provider();

            #[cfg(not(feature = "postgres_rustls_native_certs"))]
            let tls_provider = deadpool_postgres::tokio_postgres::NoTls;

            let config = config.url.parse().map_err(|e| Error::Init(PoolError::Backend(e)))?;
            Ok(Self::new(config, tls_provider))
        }
    }

    #[cfg(feature = "deadpool_redis")]
    impl DeadManager for deadpool_redis::Manager {
        fn new(config: &Config, figment: &Figment) -> Result<Self, Error<PoolError<Self::Error>>> {
            let connection_config = deadpool_redis::redis::AsyncConnectionConfig::new()
                .set_connection_timeout(None)
                .set_response_timeout(redis_response_timeout(figment)?);

            Self::new_with_config(config.url.as_str(), connection_config)
                .map_err(|e| Error::Init(PoolError::Backend(e)))
        }
    }

    #[cfg(feature = "deadpool_redis")]
    fn redis_response_timeout(
        figment: &Figment
    ) -> Result<Option<Duration>, rocket::figment::Error> {
        use rocket::serde::{Deserialize, Deserializer, de::Error};

        #[derive(Deserialize)]
        #[serde(crate = "rocket::serde")]
        struct Options {
            #[serde(default = "default_timeout", deserialize_with = "deserialize_timeout")]
            response_timeout: Option<Duration>,
        }

        fn default_timeout() -> Option<Duration> {
            Some(Duration::from_millis(500))
        }

        fn deserialize_timeout<'de, D: Deserializer<'de>>(
            de: D
        ) -> Result<Option<Duration>, D::Error> {
            let seconds = f64::deserialize(de)?;
            let timeout = Duration::try_from_secs_f64(seconds).map_err(D::Error::custom)?;
            Ok((seconds != 0.0).then_some(timeout))
        }

        Ok(figment.extract::<Options>()?.response_timeout)
    }

    #[cfg(feature = "diesel_postgres")]
    impl DeadManager for AsyncDieselConnectionManager<diesel_async::AsyncPgConnection> {
        fn new(config: &Config, _figment: &Figment) -> Result<Self, Error<PoolError<Self::Error>>> {
            #[cfg(feature = "postgres_rustls_native_certs")]
            use diesel_async::AsyncPgConnection;
            use diesel_async::pooled_connection::ManagerConfig;

            let diesel_config = ManagerConfig::default();

            #[cfg(feature = "postgres_rustls_native_certs")]
            let diesel_config = {
                let mut diesel_config = diesel_config;
                let tls_provider = super::tokio_postgres_tls_provider();

                diesel_config.custom_setup = Box::new(move |url| {
                    let tls_provider = tls_provider.clone();
                    Box::pin(async move {
                        let (client, conn) = tokio_postgres::connect(url, tls_provider)
                            .await
                            .map_err(|e| diesel::ConnectionError::BadConnection(e.to_string()))?;

                        AsyncPgConnection::try_from_client_and_connection(client, conn).await
                    }) as std::pin::Pin<Box<_>>
                });

                diesel_config
            };

            Ok(Self::new_with_config(config.url.as_str(), diesel_config))
        }
    }

    #[cfg(feature = "diesel_mysql")]
    impl DeadManager for AsyncDieselConnectionManager<diesel_async::AsyncMysqlConnection> {
        fn new(config: &Config, _figment: &Figment) -> Result<Self, Error<PoolError<Self::Error>>> {
            Ok(Self::new(config.url.as_str()))
        }
    }

    #[rocket::async_trait]
    impl<M: DeadManager, C: From<Object<M>>> crate::Pool for Pool<M, C>
        where M::Type: Send, C: Send + 'static, M::Error: std::error::Error
    {
        type Error = Error<PoolError<M::Error>>;

        type Connection = C;

        async fn init(figment: &Figment) -> Result<Self, Self::Error> {
            let config: Config = figment.extract()?;
            let manager = M::new(&config, figment)?;

            Pool::builder(manager)
                .max_size(config.max_connections)
                .wait_timeout(Some(Duration::from_secs(config.connect_timeout)))
                .create_timeout(Some(Duration::from_secs(config.connect_timeout)))
                .recycle_timeout(config.idle_timeout.map(Duration::from_secs))
                .runtime(Runtime::Tokio1)
                .build()
                .map_err(|_| Error::Init(PoolError::NoRuntimeSpecified))
        }

        async fn get(&self) -> Result<Self::Connection, Self::Error> {
            self.get().await.map_err(Error::Get)
        }

        async fn close(&self) {
            <Pool<M, C>>::close(self)
        }
    }
}

#[cfg(feature = "sqlx")]
mod sqlx {
    use sqlx::ConnectOptions;
    use crate::Configurable;
    use super::{Duration, Error, Config, Figment};
    use rocket::config::LogLevel;

    type Options<D> = <<D as sqlx::Database>::Connection as sqlx::Connection>::Options;

    // Provide specialized configuration for particular databases.
    fn specialize(__options: &mut dyn std::any::Any, __config: &Config, __figment: &Figment) {
        #[cfg(feature = "sqlx_sqlite")]
        if let Some(o) = __options.downcast_mut::<sqlx::sqlite::SqliteConnectOptions>() {
            *o = std::mem::take(o)
                .busy_timeout(Duration::from_secs(__config.connect_timeout))
                .create_if_missing(true);

            if let Some(md) = __figment.find_metadata("extensions") {
                use rocket::yansi::Paint;

                rocket::warn!("found value for deprecated SQLite config key `{}`",
                    "extensions".bold().primary());
                if let Some(ref source) = md.source {
                    rocket::warn_!("in {} {}", source.primary(), md.name);
                }

                rocket::warn_!("key has no special meaning");
            }
        }
    }

    #[rocket::async_trait]
    impl<D: sqlx::Database> crate::Pool for sqlx::Pool<D> {
        type Error = Error<sqlx::Error>;

        type Connection = sqlx::pool::PoolConnection<D>;

        async fn init(figment: &Figment) -> Result<Self, Self::Error> {
            Self::init_with(figment, |_, opts| Ok(opts)).await
        }

        async fn get(&self) -> Result<Self::Connection, Self::Error> {
            self.acquire().await.map_err(Error::Get)
        }

        async fn close(&self) {
            <sqlx::Pool<D>>::close(self).await;
        }
    }

    #[rocket::async_trait]
    impl<D: sqlx::Database> Configurable for sqlx::Pool<D> {
        type Options = (sqlx::pool::PoolOptions<D>, Options<D>);

        async fn init_with<F>(figment: &Figment, f: F) -> Result<Self, Self::Error>
            where F: FnOnce(&Config, Self::Options) -> Result<Self::Options, Self::Error> + Send
        {
            let config = figment.extract::<Config>()?;
            let mut opts = config.url.parse::<Options<D>>().map_err(Error::Init)?;
            specialize(&mut opts, &config, figment);

            opts = opts.disable_statement_logging();
            if let Ok(level) = figment.extract_inner::<LogLevel>(rocket::Config::LOG_LEVEL) {
                if !matches!(level, LogLevel::Normal | LogLevel::Off) {
                    opts = opts.log_statements(level.into())
                        .log_slow_statements(level.into(), Duration::default());
                }
            }

            let pool = sqlx::pool::PoolOptions::new()
                .max_connections(config.max_connections as u32)
                .acquire_timeout(Duration::from_secs(config.connect_timeout))
                .idle_timeout(config.idle_timeout.map(Duration::from_secs))
                .min_connections(config.min_connections.unwrap_or_default());

            let (pool, opts) = f(&config, (pool, opts))?;
            pool.connect_with(opts)
                .await
                .map_err(Error::Init)
        }
    }
}

#[cfg(feature = "mongodb")]
mod mongodb {
    use mongodb::{Client, options::ClientOptions};
    use crate::Configurable;
    use super::{Duration, Error, Config, Figment};

    #[rocket::async_trait]
    impl crate::Pool for Client {
        type Error = Error<mongodb::error::Error, std::convert::Infallible>;

        type Connection = Client;

        async fn init(figment: &Figment) -> Result<Self, Self::Error> {
            Self::init_with(figment, |_, opts| Ok(opts)).await
        }

        async fn get(&self) -> Result<Self::Connection, Self::Error> {
            Ok(self.clone())
        }

        async fn close(&self) {
            // nothing to do for mongodb
        }
    }

    #[rocket::async_trait]
    impl Configurable for Client {
        type Options = ClientOptions;

        async fn init_with<F>(figment: &Figment, f: F) -> Result<Self, Self::Error>
            where F: FnOnce(&Config, Self::Options) -> Result<Self::Options, Self::Error> + Send
        {
            let config = figment.extract::<Config>()?;
            let mut opts = ClientOptions::parse(&config.url).await.map_err(Error::Init)?;
            opts.min_pool_size = config.min_connections;
            opts.max_pool_size = Some(config.max_connections as u32);
            opts.max_idle_time = config.idle_timeout.map(Duration::from_secs);
            opts.connect_timeout = Some(Duration::from_secs(config.connect_timeout));
            opts.server_selection_timeout = Some(Duration::from_secs(config.connect_timeout));
            let opts = f(&config, opts)?;
            Client::with_options(opts).map_err(Error::Init)
        }
    }
}
