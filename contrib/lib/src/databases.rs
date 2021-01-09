//! Traits, utilities, and a macro for easy database connection pooling.
//!
//! # Overview
//!
//! This module provides traits, utilities, and a procedural macro that allows
//! you to easily connect your Rocket application to databases through
//! connection pools. A _database connection pool_ is a data structure that
//! maintains active database connections for later use in the application.
//! This implementation of connection pooling support is based on
//! [`r2d2`] and exposes connections through [request guards]. Databases are
//! individually configured through Rocket's regular configuration mechanisms: a
//! `Rocket.toml` file, environment variables, or procedurally.
//!
//! Connecting your Rocket application to a database using this library occurs
//! in three simple steps:
//!
//!   1. Configure your databases in `Rocket.toml`.
//!      (see [Configuration](#configuration))
//!   2. Associate a request guard type and fairing with each database.
//!      (see [Guard Types](#guard-types))
//!   3. Use the request guard to retrieve a connection in a handler.
//!      (see [Handlers](#handlers))
//!
//! For a list of supported databases, see [Provided Databases](#provided). This
//! support can be easily extended by implementing the [`Poolable`] trait. See
//! [Extending](#extending) for more.
//!
//! ## Example
//!
//! Before using this library, the feature corresponding to your database type
//! in `rocket_contrib` must be enabled:
//!
//! ```toml
//! [dependencies.rocket_contrib]
//! version = "0.5.0-dev"
//! default-features = false
//! features = ["diesel_sqlite_pool"]
//! ```
//!
//! See [Provided](#provided) for a list of supported database and their
//! associated feature name.
//!
//! In whichever configuration source you choose, configure a `databases`
//! dictionary with an internal dictionary for each database, here `sqlite_logs`
//! in a TOML source:
//!
//! ```toml
//! [global.databases]
//! sqlite_logs = { url = "/path/to/database.sqlite" }
//! ```
//!
//! In your application's source code, one-time:
//!
//! ```rust
//! #[macro_use] extern crate rocket;
//! #[macro_use] extern crate rocket_contrib;
//!
//! # #[cfg(feature = "diesel_sqlite_pool")]
//! # mod test {
//! use rocket_contrib::databases::diesel;
//!
//! #[database("sqlite_logs")]
//! struct LogsDbConn(diesel::SqliteConnection);
//!
//! #[launch]
//! fn rocket() -> rocket::Rocket {
//!     rocket::ignite().attach(LogsDbConn::fairing())
//! }
//! # } fn main() {}
//! ```
//!
//! Whenever a connection to the database is needed:
//!
//! ```rust
//! # #[macro_use] extern crate rocket;
//! # #[macro_use] extern crate rocket_contrib;
//! #
//! # #[cfg(feature = "diesel_sqlite_pool")]
//! # mod test {
//! # use rocket_contrib::databases::diesel;
//! #
//! # #[database("sqlite_logs")]
//! # struct LogsDbConn(diesel::SqliteConnection);
//! #
//! # type Logs = ();
//! # type Result<T> = std::result::Result<T, ()>;
//! #
//! #[get("/logs/<id>")]
//! async fn get_logs(conn: LogsDbConn, id: usize) -> Result<Logs> {
//! # /*
//!     conn.run(|c| Logs::by_id(c, id)).await
//! # */
//! # Ok(())
//! }
//! # } fn main() {}
//! ```
//!
//! # Usage
//!
//! ## Configuration
//!
//! Databases can be configured as any other values. Using the default
//! configuration provider, either via `Rocket.toml` or environment variables.
//! You can also use a custom provider.
//!
//! ### `Rocket.toml`
//!
//! To configure a database via `Rocket.toml`, add a table for each database
//! to the `databases` table where the key is a name of your choice. The table
//! should have a `url` key and, optionally, a `pool_size` key. This looks as
//! follows:
//!
//! ```toml
//! # Option 1:
//! [global.databases]
//! sqlite_db = { url = "db.sqlite" }
//!
//! # Option 2:
//! [global.databases.my_db]
//! url = "mysql://root:root@localhost/my_db"
//!
//! # With a `pool_size` key:
//! [global.databases]
//! sqlite_db = { url = "db.sqlite", pool_size = 20 }
//! ```
//!
//! The table _requires_ one key:
//!
//!   * `url` - the URl to the database
//!
//! Additionally, all configurations accept the following _optional_ keys:
//!
//!   * `pool_size` - the size of the pool, i.e., the number of connections to
//!     pool (defaults to the configured number of workers * 2)
//!
//! Additional options may be required or supported by other adapters.
//!
//! ### Procedurally
//!
//! Databases can also be configured procedurally via `rocket::custom()`.
//! The example below does just this:
//!
//! ```rust
//! # #[cfg(feature = "diesel_sqlite_pool")] {
//! use rocket::figment::{value::{Map, Value}, util::map};
//!
//! #[rocket::launch]
//! fn rocket() -> _ {
//!     let db: Map<_, Value> = map! {
//!         "url" => "db.sqlite".into(),
//!         "pool_size" => 10.into()
//!     };
//!
//!     let figment = rocket::Config::figment()
//!         .merge(("databases", map!["my_db" => db]));
//!
//!     rocket::custom(figment)
//! }
//! # rocket();
//! # }
//! ```
//!
//! ### Environment Variables
//!
//! Lastly, databases can be configured via environment variables by specifying
//! the `databases` table as detailed in the [Environment Variables
//! configuration
//! guide](https://rocket.rs/master/guide/configuration/#environment-variables):
//!
//! ```bash
//! ROCKET_DATABASES='{my_db={url="db.sqlite"}}'
//! ```
//!
//! Multiple databases can be specified in the `ROCKET_DATABASES` environment variable
//! as well by comma separating them:
//!
//! ```bash
//! ROCKET_DATABASES='{my_db={url="db.sqlite"},my_pg_db={url="postgres://root:root@localhost/my_pg_db"}}'
//! ```
//!
//! ## Guard Types
//!
//! Once a database has been configured, the `#[database]` attribute can be used
//! to tie a type in your application to a configured database. The database
//! attributes accepts a single string parameter that indicates the name of the
//! database. This corresponds to the database name set as the database's
//! configuration key.
//!
//! The macro generates a [`FromRequest`] implementation for the decorated type,
//! allowing the type to be used as a request guard. This implementation
//! retrieves a connection from the database pool or fails with a
//! `Status::ServiceUnavailable` if connecting to the database times out.
//!
//! The macro will also generate two inherent methods on the decorated type:
//!
//!   * `fn fairing() -> impl Fairing`
//!
//!      Returns a fairing that initializes the associated database connection
//!      pool.
//!
//!   * `async fn get_one(&Rocket) -> Option<Self>`
//!
//!     Retrieves a connection wrapper from the configured pool. Returns `Some`
//!     as long as `Self::fairing()` has been attached.
//!
//! The attribute can only be applied to unit-like structs with one type. The
//! internal type of the structure must implement [`Poolable`].
//!
//! ```rust
//! # extern crate rocket;
//! # #[macro_use] extern crate rocket_contrib;
//! # #[cfg(feature = "diesel_sqlite_pool")]
//! # mod test {
//! use rocket_contrib::databases::diesel;
//!
//! #[database("my_db")]
//! struct MyDatabase(diesel::SqliteConnection);
//! # }
//! ```
//!
//! Other databases can be used by specifying their respective [`Poolable`]
//! type:
//!
//! ```rust
//! # extern crate rocket;
//! # #[macro_use] extern crate rocket_contrib;
//! # #[cfg(feature = "postgres_pool")]
//! # mod test {
//! use rocket_contrib::databases::postgres;
//!
//! #[database("my_pg_db")]
//! struct MyPgDatabase(postgres::Client);
//! # }
//! ```
//!
//! The fairing returned from the generated `fairing()` method _must_ be
//! attached for the request guard implementation to succeed. Putting the pieces
//! together, a use of the `#[database]` attribute looks as follows:
//!
//! ```rust
//! # #[macro_use] extern crate rocket;
//! # #[macro_use] extern crate rocket_contrib;
//! #
//! # #[cfg(feature = "diesel_sqlite_pool")] {
//! # use rocket::figment::{value::{Map, Value}, util::map};
//! use rocket_contrib::databases::diesel;
//!
//! #[database("my_db")]
//! struct MyDatabase(diesel::SqliteConnection);
//!
//! #[launch]
//! fn rocket() -> _ {
//! #   let db: Map<_, Value> = map![
//! #        "url" => "db.sqlite".into(), "pool_size" => 10.into()
//! #   ];
//! #   let figment = rocket::Config::figment().merge(("databases", map!["my_db" => db]));
//!     rocket::custom(figment).attach(MyDatabase::fairing())
//! }
//! # }
//! ```
//!
//! ## Handlers
//!
//! Finally, use your type as a request guard in a handler to retrieve a
//! connection wrapper for the database:
//!
//! ```rust
//! # #[macro_use] extern crate rocket;
//! # #[macro_use] extern crate rocket_contrib;
//! #
//! # #[cfg(feature = "diesel_sqlite_pool")]
//! # mod test {
//! # use rocket_contrib::databases::diesel;
//! #[database("my_db")]
//! struct MyDatabase(diesel::SqliteConnection);
//!
//! #[get("/")]
//! fn my_handler(conn: MyDatabase) {
//!     // ...
//! }
//! # }
//! ```
//!
//! A connection can be retrieved and used with the `run()` method:
//!
//! ```rust
//! # #[macro_use] extern crate rocket;
//! # #[macro_use] extern crate rocket_contrib;
//! #
//! # #[cfg(feature = "diesel_sqlite_pool")]
//! # mod test {
//! # use rocket_contrib::databases::diesel;
//! # type Data = ();
//! #[database("my_db")]
//! struct MyDatabase(diesel::SqliteConnection);
//!
//! fn load_from_db(conn: &diesel::SqliteConnection) -> Data {
//!     // Do something with connection, return some data.
//!     # ()
//! }
//!
//! #[get("/")]
//! async fn my_handler(mut conn: MyDatabase) -> Data {
//!     conn.run(|c| load_from_db(c)).await
//! }
//! # }
//! ```
//!
//! # Database Support
//!
//! Built-in support is provided for many popular databases and drivers. Support
//! can be easily extended by [`Poolable`] implementations.
//!
//! ## Provided
//!
//! The list below includes all presently supported database adapters and their
//! corresponding [`Poolable`] type.
//!
// Note: Keep this table in sync with site/guite/6-state.md
//! | Kind     | Driver                | Version   | `Poolable` Type                | Feature                |
//! |----------|-----------------------|-----------|--------------------------------|------------------------|
//! | MySQL    | [Diesel]              | `1`       | [`diesel::MysqlConnection`]    | `diesel_mysql_pool`    |
//! | MySQL    | [`rust-mysql-simple`] | `18`      | [`mysql::Conn`]                | `mysql_pool`           |
//! | Postgres | [Diesel]              | `1`       | [`diesel::PgConnection`]       | `diesel_postgres_pool` |
//! | Postgres | [Rust-Postgres]       | `0.17`    | [`postgres::Client`]           | `postgres_pool`        |
//! | Sqlite   | [Diesel]              | `1`       | [`diesel::SqliteConnection`]   | `diesel_sqlite_pool`   |
//! | Sqlite   | [`Rusqlite`]          | `0.23`    | [`rusqlite::Connection`]       | `sqlite_pool`          |
//! | Memcache | [`memcache`]          | `0.15`    | [`memcache::Client`]           | `memcache_pool`        |
//!
//! [Diesel]: https://diesel.rs
//! [`rusqlite::Connection`]: https://docs.rs/rusqlite/0.23.0/rusqlite/struct.Connection.html
//! [`diesel::SqliteConnection`]: http://docs.diesel.rs/diesel/prelude/struct.SqliteConnection.html
//! [`postgres::Client`]: https://docs.rs/postgres/0.17/postgres/struct.Client.html
//! [`diesel::PgConnection`]: http://docs.diesel.rs/diesel/pg/struct.PgConnection.html
//! [`mysql::Conn`]: https://docs.rs/mysql/18/mysql/struct.Conn.html
//! [`diesel::MysqlConnection`]: http://docs.diesel.rs/diesel/mysql/struct.MysqlConnection.html
//! [`Rusqlite`]: https://github.com/jgallagher/rusqlite
//! [Rust-Postgres]: https://github.com/sfackler/rust-postgres
//! [`rust-mysql-simple`]: https://github.com/blackbeam/rust-mysql-simple
//! [`diesel::PgConnection`]: http://docs.diesel.rs/diesel/pg/struct.PgConnection.html
//! [`memcache`]: https://github.com/aisk/rust-memcache
//! [`memcache::Client`]: https://docs.rs/memcache/0.15/memcache/struct.Client.html
//!
//! The above table lists all the supported database adapters in this library.
//! In order to use particular `Poolable` type that's included in this library,
//! you must first enable the feature listed in the "Feature" column. The
//! interior type of your decorated database type should match the type in the
//! "`Poolable` Type" column.
//!
//! ## Extending
//!
//! Extending Rocket's support to your own custom database adapter (or other
//! database-like struct that can be pooled by `r2d2`) is as easy as
//! implementing the [`Poolable`] trait. See the documentation for [`Poolable`]
//! for more details on how to implement it.
//!
//! [`FromRequest`]: rocket::request::FromRequest
//! [request guards]: rocket::request::FromRequest
//! [`Poolable`]: crate::databases::Poolable

pub extern crate r2d2;

#[cfg(any(
    feature = "diesel_sqlite_pool",
    feature = "diesel_postgres_pool",
    feature = "diesel_mysql_pool"
))]
pub extern crate diesel;

use std::marker::PhantomData;
use std::sync::Arc;

use rocket::fairing::{AdHoc, Fairing};
use rocket::request::{Request, Outcome, FromRequest};
use rocket::outcome::IntoOutcome;
use rocket::http::Status;

use rocket::tokio::sync::{OwnedSemaphorePermit, Semaphore, Mutex};
use rocket::tokio::time::timeout;

use self::r2d2::ManageConnection;

#[doc(hidden)] pub use rocket_contrib_codegen::*;

#[cfg(feature = "postgres_pool")] pub extern crate postgres;
#[cfg(feature = "postgres_pool")] pub extern crate r2d2_postgres;

#[cfg(feature = "mysql_pool")] pub extern crate mysql;
#[cfg(feature = "mysql_pool")] pub extern crate r2d2_mysql;

#[cfg(feature = "sqlite_pool")] pub extern crate rusqlite;
#[cfg(feature = "sqlite_pool")] pub extern crate r2d2_sqlite;

#[cfg(feature = "memcache_pool")] pub extern crate memcache;
#[cfg(feature = "memcache_pool")] pub extern crate r2d2_memcache;

/// A base `Config` for any `Poolable` type.
///
/// For the following configuration:
///
/// ```toml
/// [global.databases.my_database]
/// url = "postgres://root:root@localhost/my_database"
/// pool_size = 10
/// timeout = 5
/// ```
///
/// ...`Config::from("my_database", rocket)` would return the following struct:
///
/// ```rust
/// # use rocket_contrib::databases::Config;
/// Config {
///     url: "postgres://root:root@localhost/my_database".into(),
///     pool_size: 10,
///     timeout: 5
/// };
/// ```
///
/// If you want to implement your own custom database adapter (or other
/// database-like struct that can be pooled by `r2d2`) and need some more
/// configurations options, you may need to define a custom `Config` struct.
/// Note, however, that the configuration values in `Config` are required.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Config {
    /// Connection URL specified in the Rocket configuration.
    pub url: String,
    /// Initial pool size. Defaults to the number of Rocket workers * 2.
    pub pool_size: u32,
    /// How long to wait, in seconds, for a new connection before timing out.
    /// Defaults to `5`.
    // FIXME: Use `time`.
    pub timeout: u8,
}

use serde::{Serialize, Deserialize};
use rocket::figment::{self, Figment, providers::Serialized};

impl Config {
    /// Retrieves the database configuration for the database named `name`.
    ///
    /// This function is primarily used by the generated code from the
    /// `#[database]` attribute.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[cfg(feature = "diesel_sqlite_pool")] {
    /// # use rocket::figment::{Figment, providers::{Format, Toml}};
    /// // Assume that these are the contents of `Rocket.toml`:
    /// # let toml = Toml::string(r#"
    /// [global.databases]
    /// my_db = { url = "db/db.sqlite", pool_size = 25 }
    /// my_other_db = { url = "mysql://root:root@localhost/database" }
    /// # "#).nested();
    ///
    /// use rocket_contrib::databases::Config;
    ///
    /// fn pool(rocket: &rocket::Rocket) {
    ///     let config = Config::from("my_db", rocket).unwrap();
    ///     assert_eq!(config.url, "db/db.sqlite");
    ///     assert_eq!(config.pool_size, 25);
    ///
    ///     let config = Config::from("my_other_db", rocket).unwrap();
    ///     assert_eq!(config.url, "mysql://root:root@localhost/database");
    ///     assert_eq!(config.pool_size, (rocket.config().workers * 2) as u32);
    ///
    ///     let config = Config::from("unknown_db", rocket);
    ///     assert!(config.is_err())
    /// }
    /// #
    /// # let config = Figment::from(rocket::Config::default()).merge(toml);
    /// # let rocket = rocket::custom(config);
    /// # pool(&rocket);
    /// # }
    /// ```
    pub fn from(db_name: &str, rocket: &rocket::Rocket) -> Result<Config, figment::Error> {
        let db_key = format!("databases.{}", db_name);
        let key = |name: &str| format!("{}.{}", db_key, name);
        Figment::from(rocket.figment())
            .merge(Serialized::default(&key("pool_size"), rocket.config().workers * 2))
            .merge(Serialized::default(&key("timeout"), 5))
            .extract_inner::<Self>(&db_key)
    }
}

/// A wrapper around `r2d2::Error`s or a custom database error type.
///
/// This type is only relevant to implementors of the [`Poolable`] trait. See
/// the [`Poolable`] documentation for more information on how to use this type.
#[derive(Debug)]
pub enum Error<T> {
    /// A custom error of type `T`.
    Custom(T),
    /// An error occurred while initializing an `r2d2` pool.
    Pool(r2d2::Error),
    /// An error occurred while extracting a `figment` configuration.
    Config(figment::Error),
}

impl<T> From<figment::Error> for Error<T> {
    fn from(error: figment::Error) -> Self {
        Error::Config(error)
    }
}

impl<T> From<r2d2::Error> for Error<T> {
    fn from(error: r2d2::Error) -> Self {
        Error::Pool(error)
    }
}

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
///   * `mysql::Conn`
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
/// #     use rocket_contrib::databases::r2d2;
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
/// use rocket_contrib::databases::{r2d2, Error, Config, Poolable, PoolResult};
///
/// impl Poolable for foo::Connection {
///     type Manager = foo::ConnectionManager;
///     type Error = foo::Error;
///
///     fn pool(db_name: &str, rocket: &rocket::Rocket) -> PoolResult<Self> {
///         let config = Config::from(db_name, rocket)?;
///         let manager = foo::ConnectionManager::new(&config.url).map_err(Error::Custom)?;
///         Ok(r2d2::Pool::builder().max_size(config.pool_size).build(manager)?)
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
    fn pool(db_name: &str, rocket: &rocket::Rocket) -> PoolResult<Self>;
}

/// A type alias for the return type of [`Poolable::pool()`].
#[allow(type_alias_bounds)]
pub type PoolResult<P: Poolable> = Result<r2d2::Pool<P::Manager>, Error<P::Error>>;

#[cfg(feature = "diesel_sqlite_pool")]
impl Poolable for diesel::SqliteConnection {
    type Manager = diesel::r2d2::ConnectionManager<diesel::SqliteConnection>;
    type Error = std::convert::Infallible;

    fn pool(db_name: &str, rocket: &rocket::Rocket) -> PoolResult<Self> {
        let config = Config::from(db_name, rocket)?;
        let manager = diesel::r2d2::ConnectionManager::new(&config.url);
        Ok(r2d2::Pool::builder().max_size(config.pool_size).build(manager)?)
    }
}

#[cfg(feature = "diesel_postgres_pool")]
impl Poolable for diesel::PgConnection {
    type Manager = diesel::r2d2::ConnectionManager<diesel::PgConnection>;
    type Error = std::convert::Infallible;

    fn pool(db_name: &str, rocket: &rocket::Rocket) -> PoolResult<Self> {
        let config = Config::from(db_name, rocket)?;
        let manager = diesel::r2d2::ConnectionManager::new(&config.url);
        Ok(r2d2::Pool::builder().max_size(config.pool_size).build(manager)?)
    }
}

#[cfg(feature = "diesel_mysql_pool")]
impl Poolable for diesel::MysqlConnection {
    type Manager = diesel::r2d2::ConnectionManager<diesel::MysqlConnection>;
    type Error = std::convert::Infallible;

    fn pool(db_name: &str, rocket: &rocket::Rocket) -> PoolResult<Self> {
        let config = Config::from(db_name, rocket)?;
        let manager = diesel::r2d2::ConnectionManager::new(&config.url);
        Ok(r2d2::Pool::builder().max_size(config.pool_size).build(manager)?)
    }
}

// TODO: Add a feature to enable TLS in `postgres`; parse a suitable `config`.
#[cfg(feature = "postgres_pool")]
impl Poolable for postgres::Client {
    type Manager = r2d2_postgres::PostgresConnectionManager<postgres::tls::NoTls>;
    type Error = postgres::Error;

    fn pool(db_name: &str, rocket: &rocket::Rocket) -> PoolResult<Self> {
        let config = Config::from(db_name, rocket)?;
        let url = config.url.parse().map_err(Error::Custom)?;
        let manager = r2d2_postgres::PostgresConnectionManager::new(url, postgres::tls::NoTls);
        Ok(r2d2::Pool::builder().max_size(config.pool_size).build(manager)?)
    }
}

#[cfg(feature = "mysql_pool")]
impl Poolable for mysql::Conn {
    type Manager = r2d2_mysql::MysqlConnectionManager;
    type Error = std::convert::Infallible;

    fn pool(db_name: &str, rocket: &rocket::Rocket) -> PoolResult<Self> {
        let config = Config::from(db_name, rocket)?;
        let opts = mysql::OptsBuilder::from_opts(&config.url);
        let manager = r2d2_mysql::MysqlConnectionManager::new(opts);
        Ok(r2d2::Pool::builder().max_size(config.pool_size).build(manager)?)
    }
}

#[cfg(feature = "sqlite_pool")]
impl Poolable for rusqlite::Connection {
    type Manager = r2d2_sqlite::SqliteConnectionManager;
    type Error = std::convert::Infallible;

    fn pool(db_name: &str, rocket: &rocket::Rocket) -> PoolResult<Self> {
        let config = Config::from(db_name, rocket)?;
        let manager = r2d2_sqlite::SqliteConnectionManager::file(&*config.url);
        Ok(r2d2::Pool::builder().max_size(config.pool_size).build(manager)?)
    }
}

#[cfg(feature = "memcache_pool")]
impl Poolable for memcache::Client {
    type Manager = r2d2_memcache::MemcacheConnectionManager;
    // Unused, but we might want it in the future without a breaking change.
    type Error = memcache::MemcacheError;

    fn pool(db_name: &str, rocket: &rocket::Rocket) -> PoolResult<Self> {
        let config = Config::from(db_name, rocket)?;
        let manager = r2d2_memcache::MemcacheConnectionManager::new(&*config.url);
        Ok(r2d2::Pool::builder().max_size(config.pool_size).build(manager)?)
    }
}

/// Unstable internal details of generated code for the #[database] attribute.
///
/// This type is implemented here instead of in generated code to ensure all
/// types are properly checked.
#[doc(hidden)]
pub struct ConnectionPool<K, C: Poolable> {
    config: Config,
    // This is an 'Option' so that we can drop the pool in a 'spawn_blocking'.
    pool: Option<r2d2::Pool<C::Manager>>,
    semaphore: Arc<Semaphore>,
    _marker: PhantomData<fn() -> K>,
}

impl<K, C: Poolable> Clone for ConnectionPool<K, C> {
    fn clone(&self) -> Self {
        ConnectionPool {
            config: self.config.clone(),
            pool: self.pool.clone(),
            semaphore: self.semaphore.clone(),
            _marker: PhantomData
        }
    }
}

/// Unstable internal details of generated code for the #[database] attribute.
///
/// This type is implemented here instead of in generated code to ensure all
/// types are properly checked.
#[doc(hidden)]
pub struct Connection<K, C: Poolable> {
    connection: Arc<Mutex<Option<r2d2::PooledConnection<C::Manager>>>>,
    permit: Option<OwnedSemaphorePermit>,
    _marker: PhantomData<fn() -> K>,
}

// A wrapper around spawn_blocking that propagates panics to the calling code.
async fn run_blocking<F, R>(job: F) -> R
    where F: FnOnce() -> R + Send + 'static, R: Send + 'static,
{
    match tokio::task::spawn_blocking(job).await {
        Ok(ret) => ret,
        Err(e) => match e.try_into_panic() {
            Ok(panic) => std::panic::resume_unwind(panic),
            Err(_) => unreachable!("spawn_blocking tasks are never cancelled"),
        }
    }
}

macro_rules! dberr {
    ($msg:literal, $db_name:expr, $efmt:literal, $error:expr, $rocket:expr) => ({
        rocket::error!(concat!("database ", $msg, " error for pool named `{}`"), $db_name);
        error_!($efmt, $error);
        return Err($rocket);
    });
}

impl<K: 'static, C: Poolable> ConnectionPool<K, C> {
    pub fn fairing(fairing_name: &'static str, db: &'static str) -> impl Fairing {
        AdHoc::on_attach(fairing_name, move |rocket| async move {
            let config = match Config::from(db, &rocket) {
                Ok(config) => config,
                Err(e) => dberr!("config", db, "{}", e, rocket),
            };

            let pool_size = config.pool_size;
            match C::pool(db, &rocket) {
                Ok(pool) => Ok(rocket.manage(ConnectionPool::<K, C> {
                    config,
                    pool: Some(pool),
                    semaphore: Arc::new(Semaphore::new(pool_size as usize)),
                    _marker: PhantomData,
                })),
                Err(Error::Config(e)) => dberr!("config", db, "{}", e, rocket),
                Err(Error::Pool(e)) => dberr!("pool init", db, "{}", e, rocket),
                Err(Error::Custom(e)) => dberr!("pool manager", db, "{:?}", e, rocket),
            }
        })
    }

    async fn get(&self) -> Result<Connection<K, C>, ()> {
        let duration = std::time::Duration::from_secs(self.config.timeout as u64);
        let permit = match timeout(duration, self.semaphore.clone().acquire_owned()).await {
            Ok(p) => p,
            Err(_) => {
                error_!("database connection retrieval timed out");
                return Err(());
            }
        };

        let pool = self.pool.as_ref().cloned()
            .expect("internal invariant broken: self.pool is Some");

        match run_blocking(move || pool.get_timeout(duration)).await {
            Ok(c) => Ok(Connection {
                connection: Arc::new(Mutex::new(Some(c))),
                permit: Some(permit),
                _marker: PhantomData,
            }),
            Err(e) => {
                error_!("failed to get a database connection: {}", e);
                Err(())
            }
        }
    }

    #[inline]
    pub async fn get_one(rocket: &rocket::Rocket) -> Option<Connection<K, C>> {
        match rocket.state::<Self>() {
            Some(pool) => pool.get().await.ok(),
            None => None
        }
    }

    #[inline]
    pub async fn get_pool(rocket: &rocket::Rocket) -> Option<Self> {
        rocket.state::<Self>().map(|pool| pool.clone())
    }
}

impl<K: 'static, C: Poolable> Connection<K, C> {
    #[inline]
    pub async fn run<F, R>(&self, f: F) -> R
        where F: FnOnce(&mut C) -> R + Send + 'static,
              R: Send + 'static,
    {
        let mut connection = self.connection.clone().lock_owned().await;
        run_blocking(move || {
            let conn = connection.as_mut()
                .expect("internal invariant broken: self.connection is Some");
            f(conn)
        }).await
    }
}

impl<K, C: Poolable> Drop for Connection<K, C> {
    fn drop(&mut self) {
        let connection = self.connection.clone();
        let permit = self.permit.take();
        tokio::spawn(async move {
            let mut connection = connection.lock_owned().await;
            tokio::task::spawn_blocking(move || {
                if let Some(conn) = connection.take() {
                    drop(conn);
                }

                // Explicitly dropping the permit here so that it's only
                // released after the connection is.
                drop(permit);
            })
        });
    }
}

impl<K, C: Poolable> Drop for ConnectionPool<K, C> {
    fn drop(&mut self) {
        let pool = self.pool.take();
        tokio::task::spawn_blocking(move || drop(pool));
    }
}

#[rocket::async_trait]
impl<'a, 'r, K: 'static, C: Poolable> FromRequest<'a, 'r> for Connection<K, C> {
    type Error = ();

    #[inline]
    async fn from_request(request: &'a Request<'r>) -> Outcome<Self, ()> {
        match request.managed_state::<ConnectionPool<K, C>>() {
            Some(c) => c.get().await.into_outcome(Status::ServiceUnavailable),
            None => {
                error_!("Missing database fairing for `{}`", std::any::type_name::<K>());
                Outcome::Failure((Status::InternalServerError, ()))
            }
        }
    }
}
