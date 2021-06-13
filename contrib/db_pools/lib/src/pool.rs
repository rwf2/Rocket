use rocket::async_trait;

/// This trait is implemented on connection pool types that can be used with the
/// [`Database`] derive macro.
///
/// `Pool` determines how the connection pool is initialized from configuration,
/// such as a connection string and optional pool size, along with the returned
/// `Connection` type.
///
/// Implementations of this trait should use `async_trait`.
///
/// ## Example
///
/// ```
/// #[derive(Debug)]
/// struct Error { /* ... */ }
/// # impl std::fmt::Display for Error {
/// #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
/// #         unimplemented!("example")
/// #     }
/// # }
/// # impl std::error::Error for Error { }
///
/// struct Pool { /* ... */ }
/// struct Connection { /* .. */ }
///
/// #[rocket::async_trait]
/// impl rocket_db_pools::Pool for Pool {
///     type Connection = Connection;
///     type Config = rocket_db_pools::Config;
///     type InitError = Error;
///     type GetError = Error;
///
///     async fn initialize(config: Self::Config) -> Result<Self, Self::InitError> {
///         unimplemented!("example")
///     }
///
///     async fn get(&self) -> Result<Connection, Self::GetError> {
///         unimplemented!("example")
///     }
/// }
/// ```
///
/// [`Database`]: crate::Database
#[async_trait]
pub trait Pool: Sized + Send + Sync + 'static {
    /// The type returned by get().
    type Connection;

    /// The configuration type this database pool expects.
    type Config: rocket::serde::DeserializeOwned + Send;

    /// The error type returned by `initialize`.
    type InitError: std::error::Error;

    /// The error type returned by `get`.
    type GetError: std::error::Error;

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
    ///
    /// [`Config`]: crate::Config
    async fn initialize(config: Self::Config) -> Result<Self, Self::InitError>;

    /// Asynchronously gets a connection from the factory or pool.
    ///
    /// ## Errors
    ///
    /// This method returns an error if a connection could not be retrieved,
    /// such as a preconfigured timeout elapsing or when the database server is
    /// unavailable.
    async fn get(&self) -> Result<Self::Connection, Self::GetError>;
}

#[cfg(feature = "deadpool-postgres")]
#[async_trait]
impl crate::Pool for deadpool_postgres::Pool {
    type Connection = deadpool_postgres::Client;
    type Config = crate::Config;
    type InitError = crate::Error<deadpool_postgres::tokio_postgres::Error>;
    type GetError = deadpool_postgres::PoolError;

    async fn initialize(config: Self::Config) -> std::result::Result<Self, Self::InitError> {
        // TODO: don't default to 10
        let pool_size = config.pool_size_or_default(10)?;

        let manager = deadpool_postgres::Manager::new(
            config.url.parse().map_err(crate::Error::Db)?,
            // TODO: add TLS support in config
            deadpool_postgres::tokio_postgres::NoTls,
        );
        Ok(deadpool_postgres::Pool::new(manager, pool_size))
    }

    async fn get(&self) -> Result<Self::Connection, Self::GetError> {
        self.get().await
    }
}

#[cfg(feature = "deadpool-redis")]
#[async_trait]
impl crate::Pool for deadpool_redis::Pool {
    type Connection = deadpool_redis::ConnectionWrapper;
    type Config = crate::Config;
    type InitError = crate::Error<deadpool_redis::redis::RedisError>;
    type GetError = deadpool_redis::PoolError;

    async fn initialize(config: Self::Config) -> std::result::Result<Self, Self::InitError> {
        // TODO: don't default to 10
        let pool_size = config.pool_size_or_default(10)?;

        Ok(deadpool_redis::Pool::new(
            deadpool_redis::Manager::new(config.url).map_err(crate::Error::Db)?,
            pool_size,
        ))
    }

    async fn get(&self) -> Result<Self::Connection, Self::GetError> {
        self.get().await
    }
}

#[cfg(feature = "mongodb")]
#[async_trait]
impl crate::Pool for mongodb::Client {
    type Connection = mongodb::Client;
    type Config = crate::Config;
    type InitError = crate::Error<mongodb::error::Error>;
    type GetError = std::convert::Infallible;

    async fn initialize(config: Self::Config) -> Result<Self, Self::InitError> {
        mongodb::Client::with_uri_str(&config.url)
            .await
            .map_err(crate::Error::Db)
    }

    async fn get(&self) -> Result<Self::Connection, Self::GetError> {
        Ok(self.clone())
    }
}

#[cfg(feature = "mysql_async")]
#[async_trait]
impl crate::Pool for mysql_async::Pool {
    type Connection = mysql_async::Conn;
    type Config = crate::Config;
    type InitError = crate::Error<mysql_async::Error>;
    type GetError = mysql_async::Error;

    async fn initialize(config: Self::Config) -> std::result::Result<Self, Self::InitError> {
        mysql_async::Pool::from_url(config.url).map_err(crate::Error::Db)
    }

    async fn get(&self) -> std::result::Result<Self::Connection, Self::GetError> {
        self.get_conn().await
    }
}

#[cfg(feature = "sqlx_mysql")]
#[async_trait]
impl crate::Pool for sqlx::MySqlPool {
    type Connection = sqlx::pool::PoolConnection<sqlx::MySql>;
    type Config = crate::Config;
    type InitError = crate::Error<sqlx::Error>;
    type GetError = sqlx::Error;

    async fn initialize(config: Self::Config) -> std::result::Result<Self, Self::InitError> {
        use sqlx::ConnectOptions;

        let mut opts = config.url.parse::<sqlx::mysql::MySqlConnectOptions>()
            .map_err(crate::Error::Db)?;
        opts.disable_statement_logging();
        sqlx::Pool::connect(&config.url).await.map_err(crate::Error::Db)
    }

    async fn get(&self) -> std::result::Result<Self::Connection, Self::GetError> {
        self.acquire().await
    }
}

#[cfg(feature = "sqlx_postgres")]
#[async_trait]
impl crate::Pool for sqlx::PgPool {
    type Connection = sqlx::pool::PoolConnection<sqlx::Postgres>;
    type Config = crate::Config;
    type InitError = crate::Error<sqlx::Error>;
    type GetError = sqlx::Error;

    async fn initialize(config: Self::Config) -> std::result::Result<Self, Self::InitError> {
        use sqlx::ConnectOptions;

        let mut opts = config.url.parse::<sqlx::postgres::PgConnectOptions>()
            .map_err(crate::Error::Db)?;
        opts.disable_statement_logging();
        sqlx::Pool::connect_with(opts).await.map_err(crate::Error::Db)
    }

    async fn get(&self) -> std::result::Result<Self::Connection, Self::GetError> {
        self.acquire().await
    }
}

#[cfg(feature = "sqlx_sqlite")]
#[async_trait]
impl crate::Pool for sqlx::SqlitePool {
    type Connection = sqlx::pool::PoolConnection<sqlx::Sqlite>;
    type Config = crate::Config;
    type InitError = crate::Error<sqlx::Error>;
    type GetError = sqlx::Error;

    async fn initialize(config: Self::Config) -> std::result::Result<Self, Self::InitError> {
        use sqlx::ConnectOptions;

        let mut opts = config.url.parse::<sqlx::sqlite::SqliteConnectOptions>()
            .map_err(crate::Error::Db)?
            .create_if_missing(true);
        opts.disable_statement_logging();

        sqlx::Pool::connect_with(opts).await.map_err(crate::Error::Db)
    }

    async fn get(&self) -> std::result::Result<Self::Connection, Self::GetError> {
        self.acquire().await
    }
}
