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
///         // Return the fully intialized pool.
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

    /// Constructs a pool from a [Value](rocket::figment::value::Value).
    ///
    /// It is up to each implementor of `Pool` to define its accepted
    /// configuration value(s) via the `Config` associated type.  Most
    /// integrations provided in `rocket_db_pools` use [`Config`], which
    /// accepts a (required) `url` and an (optional) `pool_size`.
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

#[cfg(feature = "deadpool")]
mod deadpool_postgres {
    use deadpool::{managed::{Manager, Pool, PoolError, Object, BuildError}, Runtime};
    use super::{Duration, Error, Config, Figment};

    pub trait DeadManager: Manager + Sized + Send + Sync + 'static {
        fn new(config: &Config) -> Result<Self, Self::Error>;
    }

    #[cfg(feature = "deadpool_postgres")]
    impl DeadManager for deadpool_postgres::Manager {
        fn new(config: &Config) -> Result<Self, Self::Error> {
            Ok(Self::new(config.url.parse()?, deadpool_postgres::tokio_postgres::NoTls))
        }
    }

    #[cfg(feature = "deadpool_redis")]
    impl DeadManager for deadpool_redis::Manager {
        fn new(config: &Config) -> Result<Self, Self::Error> {
            Self::new(config.url.as_str())
        }
    }

    #[rocket::async_trait]
    impl<M: DeadManager, C: From<Object<M>>> crate::Pool for Pool<M, C>
        where M::Type: Send, C: Send + Sync + 'static, M::Error: std::error::Error
    {
        type Error = Error<BuildError<M::Error>, PoolError<M::Error>>;

        type Connection = C;

        async fn init(figment: &Figment) -> Result<Self, Self::Error> {
            let config: Config = figment.extract()?;
            let manager = M::new(&config).map_err(|e| Error::Init(BuildError::Backend(e)))?;

            Pool::builder(manager)
                .max_size(config.max_connections)
                .wait_timeout(Some(Duration::from_secs(config.connect_timeout)))
                .create_timeout(Some(Duration::from_secs(config.connect_timeout)))
                .recycle_timeout(config.idle_timeout.map(Duration::from_secs))
                .runtime(Runtime::Tokio1)
                .build()
                .map_err(Error::Init)
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
    use super::{Duration, Error, Config, Figment};
    use rocket::config::LogLevel;

    type Options<D> = <<D as sqlx::Database>::Connection as sqlx::Connection>::Options;

    // Provide specialized configuration for particular databases.
    fn specialize(__options: &mut dyn std::any::Any, __config: &Config) {
        #[cfg(feature = "sqlx_sqlite")]
        if let Some(o) = __options.downcast_mut::<sqlx::sqlite::SqliteConnectOptions>() {
            *o = std::mem::take(o)
                .busy_timeout(Duration::from_secs(__config.connect_timeout))
                .create_if_missing(true);
        }
    }

    #[rocket::async_trait]
    impl<D: sqlx::Database> crate::Pool for sqlx::Pool<D> {
        type Error = Error<sqlx::Error>;

        type Connection = sqlx::pool::PoolConnection<D>;

        async fn init(figment: &Figment) -> Result<Self, Self::Error> {
            let config = figment.extract::<Config>()?;
            let mut opts = config.url.parse::<Options<D>>().map_err(Error::Init)?;
            specialize(&mut opts, &config);

            opts.disable_statement_logging();
            if let Ok(level) = figment.extract_inner::<LogLevel>(rocket::Config::LOG_LEVEL) {
                if !matches!(level, LogLevel::Normal | LogLevel::Off) {
                    opts.log_statements(level.into())
                        .log_slow_statements(level.into(), Duration::default());
                }
            }

            sqlx::pool::PoolOptions::new()
                .max_connections(config.max_connections as u32)
                .acquire_timeout(Duration::from_secs(config.connect_timeout))
                .idle_timeout(config.idle_timeout.map(Duration::from_secs))
                .min_connections(config.min_connections.unwrap_or_default())
                .connect_with(opts)
                .await
                .map_err(Error::Init)
        }

        async fn get(&self) -> Result<Self::Connection, Self::Error> {
            self.acquire().await.map_err(Error::Get)
        }

        async fn close(&self) {
            <sqlx::Pool<D>>::close(self).await;
        }
    }
}

#[cfg(feature = "mongodb")]
mod mongodb {
    use mongodb::{Client, options::ClientOptions};
    use super::{Duration, Error, Config, Figment};

    #[rocket::async_trait]
    impl crate::Pool for Client {
        type Error = Error<mongodb::error::Error, std::convert::Infallible>;

        type Connection = Client;

        async fn init(figment: &Figment) -> Result<Self, Self::Error> {
            let config = figment.extract::<Config>()?;
            let mut opts = ClientOptions::parse(&config.url).await.map_err(Error::Init)?;
            opts.min_pool_size = config.min_connections;
            opts.max_pool_size = Some(config.max_connections as u32);
            opts.max_idle_time = config.idle_timeout.map(Duration::from_secs);
            opts.connect_timeout = Some(Duration::from_secs(config.connect_timeout));
            opts.server_selection_timeout = Some(Duration::from_secs(config.connect_timeout));
            Client::with_options(opts).map_err(Error::Init)
        }

        async fn get(&self) -> Result<Self::Connection, Self::Error> {
            Ok(self.clone())
        }

        async fn close(&self) {
            // nothing to do for mongodb
        }
    }
}
