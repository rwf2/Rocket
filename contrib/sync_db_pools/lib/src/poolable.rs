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
/// #          fn connect(&self) -> Result<Connection> { panic!(()) }
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

// TODO: Add a feature to enable TLS in `postgres`; parse a suitable `config`.
#[cfg(feature = "postgres_pool")]
impl Poolable for postgres::Client {
    type Manager = r2d2_postgres::PostgresConnectionManager<postgres::tls::NoTls>;
    type Error = postgres::Error;

    fn pool(db_name: &str, rocket: &Rocket<Build>) -> PoolResult<Self> {
        let config = Config::from(db_name, rocket)?;
        let url = config.url.parse().map_err(Error::Custom)?;
        let manager = r2d2_postgres::PostgresConnectionManager::new(url, postgres::tls::NoTls);
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
