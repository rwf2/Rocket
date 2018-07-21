//! # Overview
//!
//! This module provides traits, utilities, and a procedural macro that allows
//! you to easily connect your Rocket application to databases through
//! connection pools. A _database connection pool_ is a data structure that
//! maintains active database connections for later use in the application.
//! This implementation of connection pooling support is based on
//! [`r2d2`](https://crates.io/crates/r2d2) and exposes connections through
//! [request guards](../../rocket/request/trait.FromRequest.html). Databases are
//! individually configured through Rocket's regular configuration mechanisms: a
//! `Rocket.toml` file, environment variables, or procedurally.
//!
//! Connecting your Rocket application to a database using this library occurs
//! in three simple steps:
//!
//!   1. Configure your databases in `Rocket.toml`.
//! (see [Configuration](#configuration))
//!   2. Associate a request guard type and fairing with each database.
//! (see [Guard Types](#guard-types))
//!   3. Use the request guard to retrieve a connection in a handler.
//! (see [Handlers](#handlers))
//!
//! For a list of supported databases, see [Provided Databases](#provided).
//! This support can be easily extended by implementing the
//! [`Poolable`](trait.Poolable.html) trait. See [Extending](#extending)
//! for more.
//!
//! The next section provides a complete but un-detailed example of these steps
//! in actions. The sections following provide more detail for each component.
//!
//! ## Example
//!
//! Before using this library, the `database_pool` feature in `rocket_contrib`
//! must be enabled:
//!
//! ```toml
//! [dependencies.rocket_contrib]
//! version = "0.4.0-dev"
//! default-features = false
//! features = ["database_pool", "diesel_sqlite_pool"]
//! ```
//!
//! In `Rocket.toml` or the equivalent via environment variables:
//!
//! ```toml
//! [global.databases]
//! sqlite_logs = { url = "/path/to/database.sqlite" }
//! ```
//!
//! In your application's source code, one-time:
//!
//! ```rust,ignore
//! #![feature(use_extern_macros)]
//! extern crate rocket;
//! extern crate rocket_contrib;
//!
//! use rocket_contrib::databases::{database, diesel};
//!
//! #[database("sqlite_logs")]
//! struct LogsDbConn(diesel::SqliteConnection);
//!
//! fn main() {
//!     rocket::ignite()
//!        .attach(LogsDbConn::fairing())
//!        .launch();
//! }
//! ```
//!
//! Whenever a connection to the database is needed:
//!
//! ```rust,ignore
//! #[get("/logs/<id>")]
//! fn get_logs(conn: LogsDbConn, id: LogId) -> Result<Logs> {
//!     Logs::by_id(&conn, id)
//! }
//! ```
//!
//! # Usage
//!
//! ## Configuration
//!
//! There are a few ways to configure your database connection. You can use the
//! `Rocket.toml` file, you can build it yourself procedurally via the
//! `rocket::custom()` method, or through environment variables.
//!
//! ### Configuring via `Rocket.toml`
//!
//! The following examples are all valid ways of configuring your database via
//! the `Rocket.toml` file.
//!
//! The basic structure includes attaching a key to the `global.databases` table
//! and including the __required__ keys `url` and `pool_size`. Additional
//! options that can be added to the table vary by adapter and are referenced
//! below in the [Supported Databases](#provided) section.
//!
//! ```toml
//! [global.databases]
//! my_database = { url = "database.sqlite", pool_size = 10 }
//!
//! [[global.databases.other_database]]
//! url = "mysql://root:root@localhost/other_database
//! pool_size = 25
//! ```
//!
//! ### Configuring procedurally
//!
//! It's also possible to procedurally configure your database via the
//! `rocket::custom()` method. Below is an example of doing this:
//!
//! ```rust,ignore
//! extern crate rocket;
//!
//! use std::io::Error;
//! use std::collections::HashMap;
//! use rocket::config::{Config, Environment, Value};
//!
//! fn main() {
//!     let mut database_config = HashMap::new();
//!     let mut databases = HashMap::new();
//!
//!     database_config.insert("url", Value::from("database.sqlite"));
//!     databases.insert("my_db", Value::from(database_config));
//!
//!     let config = Config::build(Environment::Development)
//!         .extra("databases", databases)
//!         .finalize()
//!         .unwrap();
//!
//!     rocket::custom(config).launch();
//! }
//! ```
//!
//! ### Configuring via Environment Variable
//!
//! The final way to configure your databases is via an environment variable.
//! Following the syntax laid out in the guide on [Environment Variables](https://rocket.rs/guide/configuration/#environment-variables),
//! you can configure your database this way. Below is an example
//!
//! ```bash
//! ROCKET_DATABASES={my_db={url="db.sqlite"}}
//! ```
//!
//! ## Guard Types
//!
//! The included database support generates request guard types that can be used
//! with Rocket handlers. In order to associate a configured database with a
//! type, you need to use the `database` procedural macro:
//!
//! ```rust
//! # #![feature(use_extern_macros)]
//! # extern crate rocket;
//! # extern crate rocket_contrib;
//! # use rocket_contrib::databases::{database, diesel};
//!
//! #[database("my_db")]
//! struct MyDatabase(diesel::SqliteConnection);
//! ```
//!
//! From there, the macro will generate code to turn your defined type into a
//! valid request guard type. The interior type must have an implementation of
//! the [`Poolable` trait](trait.Poolable.html). The trait implements methods
//! on the interior type that are used by the generated code to spin up a
//! connection pool. The trait can be used to extend other connection types that
//! aren't supported in this library. See the section on [Extending](#extending)
//! for more information.
//!
//! The generated code will give your defined type two methods, `get_one` and
//! `fairing`, as well as implementations of the [`FromRequest`](../../rocket/request/trait.FromRequest.html)
//! and [`Deref`](../../std/ops/trait.Deref.html) traits.
//!
//! The `fairing` method will allow you to attach your database type to the
//! application state via the method call. You __will need__ to call the
//! `fairing` method on your type in order to be able to retrieve connections
//! in your request guards.
//!
//! Below is an example:
//!
//! ```rust,ignore
//! # #![feature(use_extern_macros)]
//! #
//! # extern crate rocket;
//! # extern crate rocket_contrib;
//! #
//! # use std::collections::HashMap;
//! # use rocket::config::{Config, Environment, Value};
//! # use rocket_contrib::databases::{database, diesel};
//! #
//! #[database("my_db")]
//! struct MyDatabase(diesel::SqliteConnection);
//!
//! fn main() {
//! #     let mut db_config = HashMap::new();
//! #     let mut databases = HashMap::new();
//! #
//! #     db_config.insert("url", Value::from("database.sqlite"));
//! #     db_config.insert("pool_size", Value::from(10));
//! #     databases.insert("my_db", Value::from(db_config));
//! #
//! #     let config = Config::build(Environment::Development)
//! #         .extra("databases", databases)
//! #         .finalize()
//! #         .unwrap();
//! #
//!     rocket::custom(config)
//!         .attach(MyDatabase::fairing()); // Required!
//!         .launch();
//! }
//! ```
//!
//! ## Handlers
//!
//! For request handlers, you should use the database type you defined in your
//! code as a request guard. Because of the `FromRequest` implementation that's
//! generated at compile-time, you can use this type in such a way. For example:
//!
//! ```rust,ignore
//! #[database("my_db")
//! struct MyDatabase(diesel::MysqlConnection);
//! ...
//! #[get("/")]
//! fn my_handler(conn: MyDatabase) {
//!     ...
//! }
//! ```
//!
//! Additionally, because of the `Deref` implementation, you can dereference
//! the database type in order to access the inner connection type. For example:
//!
//! ```rust,ignore
//! #[get("/")]
//! fn my_handler(conn: MyDatabase) {
//!     ...
//!     Thing::load(&conn);
//!     ...
//! }
//! ```
//!
//! Under the hood, the dereferencing of your type is returning the interior
//! type of your connection:
//!
//! ```rust,ignore
//! &self.0
//! ```
//!
//! This section should be simple. It should cover:
//!
//!   * The fact that `MyType` is not a request guard, and you can use it.
//!   * The `Deref` impl and what it means for using `&my_conn`.
//!
//! # Database Support
//!
//! This library provides built-in support for many popular databases and their
//! corresponding drivers. It also makes extending this support simple.
//!
//! ## Provided
//!
//! The list below includes all presently supported database adapters, their
//! corresponding [`Poolable`] type, and any special considerations for
//! configuration, if any.
//!
//! | Database Kind    | Driver                                                                | `Poolable` Type                                                                                                 | Feature                | Notes |
//! | -- ------------- | -----------------------                                               | -------------------------                                                                                       | ---------------------  | ----- |
//! | MySQL            | [Diesel](https://diesel.rs)                                           | [`diesel::MysqlConnection`](http://docs.diesel.rs/diesel/mysql/struct.MysqlConnection.html)                     | `diesel_mysql_pool`    | None  |
//! | MySQL            | [`rust-mysql-simple`](https://github.com/blackbeam/rust-mysql-simple) | [`mysql::conn`](https://docs.rs/mysql/14.0.0/mysql/struct.Conn.html)                                            | `mysql_pool`           | None  |
//! | Postgres         | [Diesel](https://diesel.rs)                                           | [`diesel::PgConnection`](http://docs.diesel.rs/diesel/pg/struct.PgConnection.html)                              | `diesel_postgres_pool` | None  |
//! | Postgres         | [Rust-Postgres](https://github.com/sfackler/rust-postgres)            | [`postgres::Connection`](https://docs.rs/postgres/0.15.2/postgres/struct.Connection.html)                       | `postgres_pool`        | None  |
//! | Sqlite           | [Diesel](https://diesel.rs)                                           | [`diesel::SqliteConnection`](http://docs.diesel.rs/diesel/prelude/struct.SqliteConnection.html)                 | `diesel_sqlite_pool`   | None  |
//! | Sqlite           | [`Rustqlite`](https://github.com/jgallagher/rusqlite)                 | [`rusqlite::Connection`](https://docs.rs/rusqlite/0.13.0/rusqlite/struct.Connection.html)                       | `sqlite_pool`          | None  |
//! | Neo4j            | [`rusted_cypher`](https://github.com/livioribeiro/rusted-cypher)      | [`rusted_cypher::GraphClient`](https://docs.rs/rusted_cypher/1.1.0/rusted_cypher/graph/struct.GraphClient.html) | `cypher_pool`          | None  |
//! | Redis            | [`Redis-rs`](https://github.com/mitsuhiko/redis-rs)                   | [`redis::Connection`](https://docs.rs/redis/0.9.0/redis/struct.Connection.html)                                 | `redis_pool`           | None  |
//!
//! ### How to use the table
//! The above table lists all the supported database adapters in this library.
//! In order to use particular `Poolable` type that's included in this library,
//! you must first enable the feature listed in the 'Feature' column. The inner
//! type you should use for your database type should be what's listed in the
//! corresponding `Poolable` Type column.
//!
//! ## Extending
//!
//! Extending Rocket's support to your own custom database adapter (or other
//! database-like struct that can be pooled by r2d2) is as easy as implementing
//! the `Poolable` trait for your own type. See the documentation for the
//! [`Poolable` trait](trait.Poolable.html) for more details on how to implement
//! it and extend your type for use with Rocket's database pooling feature.

pub extern crate r2d2;

use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use std::marker::{Send, Sized};

use rocket::config::{self, Value};

pub use rocket_contrib_codegen::database;

use self::r2d2::ManageConnection;

#[cfg(any(feature = "diesel_sqlite_pool", feature = "diesel_postgres_pool", feature = "diesel_mysql_pool"))]
pub extern crate diesel;

#[cfg(feature = "postgres_pool")]
pub extern crate postgres;
#[cfg(feature = "postgres_pool")]
pub extern crate r2d2_postgres;

#[cfg(feature = "mysql_pool")]
pub extern crate mysql;
#[cfg(feature = "mysql_pool")]
pub extern crate r2d2_mysql;

#[cfg(feature = "sqlite_pool")]
pub extern crate rusqlite;
#[cfg(feature = "sqlite_pool")]
pub extern crate r2d2_sqlite;

#[cfg(feature = "cypher_pool")]
pub extern crate rusted_cypher;
#[cfg(feature = "cypher_pool")]
pub extern crate r2d2_cypher;

#[cfg(feature = "redis_pool")]
pub extern crate redis;
#[cfg(feature = "redis_pool")]
pub extern crate r2d2_redis;

/// A struct containing database configuration options from some configuration.
///
/// For the following configuration:
///
/// ```toml
/// [[global.databases.my_database]]
/// url = "postgres://root:root@localhost/my_database
/// pool_size = 10
/// certs = "sample_cert.pem"
/// key = "key.pem"
/// ```
///
/// The following structure would be generated after calling
/// `database_config("my_database", &some_config)`:
///
/// ```ignore
/// DatabaseConfig {
///     url: "dummy_db.sqlite",
///     pool_size: 10,
///     extras: {
///         "certs": String("certs.pem"),
///         "key": String("key.pem")
///     }
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct DatabaseConfig<'a> {
    /// The connection URL specified in the Rocket configuration.
    pub url: &'a str,
    /// The size of the pool to be initialized. Defaults to the number of
    /// Rocket workers.
    pub pool_size: u32,
    /// Any extra options that are included in the configuration, **excluding**
    /// the url and pool_size.
    pub extras: BTreeMap<String, Value>,
}

/// A wrapper around `r2d2::Error`s or a custom database error type. This type
/// is mostly relevant to implementors of the [Poolable](trait.Poolable.html)
/// trait.
///
/// Example usages of this type are in the `Poolable` implementations that ship
/// with `rocket_contrib`.
#[derive(Debug)]
pub enum DbError<T> {
    /// The custom error type to wrap alongside `r2d2::Error`.
    Custom(T),
    /// The error returned by an r2d2 pool.
    PoolError(r2d2::Error),
}

/// The error type for fetching the DatabaseConfig
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseConfigError {
    /// Returned when the `[[global.databases]]` key is missing or empty from
    /// the loaded configuration.
    MissingTable,
    /// Returned when the database configuration key is missing from the active
    /// configuration.
    MissingKey,
    /// Returned when the configuration associated with the key isn't in the
    /// expected [Table](../../rocket/config/type.Table.html) format.
    MalformedConfiguration,
    /// Returned when the `url` field is missing.
    MissingUrl,
    /// Returned when the `url` field is of the wrong type.
    MalformedUrl,
    /// Returned when the `pool_size` exceeds `u32::max_value()` or is negative.
    InvalidPoolSize(i64),
}

/// This method retrieves the database configuration from the loaded
/// configuration and returns a [`DatabaseConfig`](struct.DatabaseConfig.html)
/// struct.
///
/// # Example:
///
/// Given the following configuration:
///
/// ```toml
/// [[global.databases]]
/// my_db = { url = "db/db.sqlite", pool_size = 25 }
/// my_other_db = { url = "mysql://root:root@localhost/database" }
/// ```
///
/// Calling the `database_config` method will return the
/// [`DatabaseConfig`](struct.DatabaseConfig.html) structure for any valid
/// configuration key. See the example code below.
///
/// ```rust
/// # extern crate rocket;
/// # extern crate rocket_contrib;
/// #
/// # use std::{collections::BTreeMap, mem::drop};
/// # use rocket::{fairing::AdHoc, config::{Config, Environment, Value}};
/// use rocket_contrib::databases::{database_config, DatabaseConfigError};
///
/// # let mut databases = BTreeMap::new();
/// #
/// # let mut my_db = BTreeMap::new();
/// # my_db.insert("url".to_string(), Value::from("db/db.sqlite"));
/// # my_db.insert("pool_size".to_string(), Value::from(25));
/// #
/// # let mut my_other_db = BTreeMap::new();
/// # my_other_db.insert("url".to_string(), Value::from("mysql://root:root@localhost/database"));
/// #
/// # databases.insert("my_db".to_string(), Value::from(my_db));
/// # databases.insert("my_other_db".to_string(), Value::from(my_other_db));
/// #
/// # let config = Config::build(Environment::Development).extra("databases", databases).expect("custom config okay");
/// #
/// # rocket::custom(config).attach(AdHoc::on_attach(|rocket| {
/// #     // HACK: This is a dirty hack required to be able to make this work
/// #     let thing = {
/// #        let rocket_config = rocket.config();
/// let config = database_config("my_db", rocket_config).expect("my_db config okay");
/// assert_eq!(config.url, "db/db.sqlite");
/// assert_eq!(config.pool_size, 25);
///
/// let other_config = database_config("my_other_db", rocket_config).expect("my_other_db config okay");
/// assert_eq!(other_config.url, "mysql://root:root@localhost/database");
///
/// let error = database_config("invalid_db", rocket_config).unwrap_err();
/// assert_eq!(error, DatabaseConfigError::MissingKey);
/// #
/// #         10
/// #    };
/// #
/// #     Ok(rocket)
/// # }));
/// ```
pub fn database_config<'a>(
    name: &str,
    from: &'a config::Config
) -> Result<DatabaseConfig<'a>, DatabaseConfigError> {
    // Find the first `databases` config that's a table with a key of 'name'
    // equal to `name`.
    let connection_config = from.get_table("databases")
        .map_err(|_| DatabaseConfigError::MissingTable)?
        .get(name)
        .ok_or(DatabaseConfigError::MissingKey)?
        .as_table()
        .ok_or(DatabaseConfigError::MalformedConfiguration)?;

    let maybe_url = connection_config.get("url")
        .ok_or(DatabaseConfigError::MissingUrl)?;

    let url = maybe_url.as_str().ok_or(DatabaseConfigError::MalformedUrl)?;

    let pool_size = connection_config.get("pool_size")
        .and_then(Value::as_integer)
        .unwrap_or(from.workers as i64);

    if pool_size < 1 || pool_size > u32::max_value() as i64 {
        return Err(DatabaseConfigError::InvalidPoolSize(pool_size));
    }

    let mut extras = connection_config.clone();
    extras.remove("url");
    extras.remove("pool_size");

    Ok(DatabaseConfig { url, pool_size: pool_size as u32, extras: extras })
}

impl<'a> Display for DatabaseConfigError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            DatabaseConfigError::MissingTable => {
                write!(f, "A table named `databases` was not found for this configuration")
            },
            DatabaseConfigError::MissingKey => {
                write!(f, "An entry in the `databases` table was not found for this key")
            },
            DatabaseConfigError::MalformedConfiguration => {
                write!(f, "The configuration for this database is malformed")
            }
            DatabaseConfigError::MissingUrl => {
                write!(f, "The connection URL is missing for this database")
            },
            DatabaseConfigError::MalformedUrl => {
                write!(f, "The specified connection URL is malformed")
            },
            DatabaseConfigError::InvalidPoolSize(invalid_size) => {
                write!(f, "'{}' is not a valid value for `pool_size`", invalid_size)
            },
        }
    }
}

/// Trait implemented by database adapters to allow for r2d2 connection pools to
/// be easily created.
///
/// # Provided Implementations
///
/// Rocket Contrib implements `Poolable` on several common database adapters.
/// The provided implementations are listed here.
///
/// * **diesel::MysqlConnection**
///
/// * **diesel::PgConnection**
///
/// * **diesel::SqliteConnection**
///
/// * **postgres::Connection**
///
/// * **mysql::Conn**
///
/// * **rusqlite::Connection**
///
/// * **rusted_cypher::GraphClient**
///
/// * **redis::Connection**
///
/// # Implementation Guide
///
/// As a r2d2-compatible database (or other resource) adapter provider,
/// implementing `Poolable` in your own library will enable Rocket users to
/// consume your adapter with its built-in connection pooling primitives.
///
/// ## Example
///
/// This example assumes a `FooConnectionManager` implementing the
/// `ManageConnection`trait required by r2d2. This connection manager abstracts
/// over a pool of `FooClient` connections.
///
/// Given the following definition of the client and connection manager:
///
/// ```rust,ignore
/// struct FooClient { ... };
///
/// impl FooClient {
///     pub fn new(...) -> Result<Self, foo::Error> {
///         ...
///     }
/// }
///
/// struct FooConnectionManager { ... };
///
/// impl FooConnectionManager {
///     pub fn new(...) -> Result<Self, foo::Error> {
///         ...
///     }
/// }
/// ```
///
/// In order to allow for Rocket Contrib to generate the required code to
/// automatically provision a r2d2 connection pool into application state, the
/// `Poolable` trait needs to be implemented for the connection type.
///
/// Given the above definitions, the following would be a valid implementation
/// of the `Poolable` trait:
///
/// ```rust,ignore
/// impl Poolable for FooClient {
///     type Manager = FooConnectionManager;
///     type Error = DbError<foo::Error>;
///
///     fn pool(config: DatabaseConfig) -> Result<r2d2::Pool<Self::Manager>, Self::Error> {
///         let manager = FooConnectionManager::new(config.url)
///             .map_err(DbError::Custom)?;
///
///         r2d2::Pool::builder().max_size(config.pool_size).build(manager)
///             .map_err(DbError::PoolError)
///     }
/// }
/// ```
///
/// In the above example, the connection manager is failable and returns the the
/// `FooClient`'s error type. Since the error type can diverge from a simple
/// r2d2 pool error, the [`DbError`](enum.DbError.html) wrapper is used. This
/// error type is defined as part of the associated type in the `Poolable` trait
/// definition.
///
/// Additionally, you'll notice that the `pool` method of the trait is used to
/// to create the connection manager and the pool. This method returns a
/// `Result` containing an r2d2 pool monomorphized to the `Manager` associated
/// type in the trait definition, or containing the `Error` associated type.
///
/// In the event that the connection manager isn't failable (as is the case in
/// Diesel's r2d2 connection manager, for example), the associated error type
/// for the `Poolable` implementation can simply be `r2d2::Error` as this is the
/// only error that can be returned by the `pool` method. You can refer to the
/// included implementations of `Poolable` in the `rocket_contrib::databases`
/// module for concrete examples.
///
pub trait Poolable: Send + Sized + 'static {
    /// The associated connection manager for the given connection type.
    type Manager: ManageConnection<Connection=Self>;
    /// The associated error type in the event that constructing the connection
    /// manager and/or the connection pool fails
    type Error;

    /// Creates an r2d2 connection pool from the provided Manager associated
    /// type and returns the pool or the error associated with the trait
    /// implementation.
    fn pool(config: DatabaseConfig) -> Result<r2d2::Pool<Self::Manager>, Self::Error>;
}

#[cfg(feature = "diesel_sqlite_pool")]
impl Poolable for diesel::SqliteConnection {
    type Manager = diesel::r2d2::ConnectionManager<diesel::SqliteConnection>;
    type Error = r2d2::Error;

    fn pool(config: DatabaseConfig) -> Result<r2d2::Pool<Self::Manager>, Self::Error> {
        let manager = diesel::r2d2::ConnectionManager::new(config.url);
        r2d2::Pool::builder().max_size(config.pool_size).build(manager)
    }
}

#[cfg(feature = "diesel_pg_pool")]
impl Poolable for diesel::PgConnection {
    type Manager = diesel::r2d2::ConnectionManager<diesel::PgConnection>;
    type Error = r2d2::Error;

    fn pool(config: DatabaseConfig) -> Result<r2d2::Pool<Self::Manager>, Self::Error> {
        let manager = diesel::r2d2::ConnectionManager::new(config.url);
        r2d2::Pool::builder().max_size(config.pool_size).build(manager)
    }
}

#[cfg(feature = "diesel_mysql_pool")]
impl Poolable for diesel::MysqlConnection {
    type Manager = diesel::r2d2::ConnectionManager<diesel::MysqlConnection>;
    type Error = r2d2::Error;

    fn pool(config: DatabaseConfig) -> Result<r2d2::Pool<Self::Manager>, Self::Error> {
        let manager = diesel::r2d2::ConnectionManager::new(config.url);
        r2d2::Pool::builder().max_size(config.pool_size).build(manager)
    }
}

// TODO: Come up with a way to handle TLS
#[cfg(feature = "postgres_pool")]
impl Poolable for postgres::Connection {
    type Manager = r2d2_postgres::PostgresConnectionManager;
    type Error = DbError<postgres::Error>;

    fn pool(config: DatabaseConfig) -> Result<r2d2::Pool<Self::Manager>, Self::Error> {
        let manager = r2d2_postgres::PostgresConnectionManager::new(config.url, r2d2_postgres::TlsMode::None)
            .map_err(DbError::Custom)?;

        r2d2::Pool::builder().max_size(config.pool_size).build(manager)
            .map_err(DbError::PoolError)
    }
}

#[cfg(feature = "mysql_pool")]
impl Poolable for mysql::Conn {
    type Manager = r2d2_mysql::MysqlConnectionManager;
    type Error = r2d2::Error;

    fn pool(config: DatabaseConfig) -> Result<r2d2::Pool<Self::Manager>, Self::Error> {
        let opts = mysql::OptsBuilder::from_opts(config.url);
        let manager = r2d2_mysql::MysqlConnectionManager::new(opts);
        r2d2::Pool::builder().max_size(config.pool_size).build(manager)
    }
}

#[cfg(feature = "sqlite_pool")]
impl Poolable for rusqlite::Connection {
    type Manager = r2d2_sqlite::SqliteConnectionManager;
    type Error = r2d2::Error;

    fn pool(config: DatabaseConfig) -> Result<r2d2::Pool<Self::Manager>, Self::Error> {
        let manager = r2d2_sqlite::SqliteConnectionManager::file(config.url);

        r2d2::Pool::builder().max_size(config.pool_size).build(manager)
    }
}

#[cfg(feature = "cypher_pool")]
impl Poolable for rusted_cypher::GraphClient {
    type Manager = r2d2_cypher::CypherConnectionManager;
    type Error = r2d2::Error;

    fn pool(config: DatabaseConfig) -> Result<r2d2::Pool<Self::Manager>, Self::Error> {
        let manager = r2d2_cypher::CypherConnectionManager { url: config.url.to_string() };
        r2d2::Pool::builder().max_size(config.pool_size).build(manager)
    }
}

#[cfg(feature = "redis_pool")]
impl Poolable for redis::Connection {
    type Manager = r2d2_redis::RedisConnectionManager;
    type Error = DbError<redis::RedisError>;

    fn pool(config: DatabaseConfig) -> Result<r2d2::Pool<Self::Manager>, Self::Error> {
        let manager = r2d2_redis::RedisConnectionManager::new(config.url).map_err(DbError::Custom)?;
        r2d2::Pool::builder().max_size(config.pool_size).build(manager)
            .map_err(DbError::PoolError)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use rocket::{Config, config::{Environment, Value}};
    use super::{DatabaseConfigError, database_config};

    #[test]
    fn no_database_entry_in_config_returns_error() {
        let config = Config::build(Environment::Development)
            .finalize()
            .unwrap();
        let database_config_result = database_config("dummy_db", &config);

        assert_eq!(Err(DatabaseConfigError::MissingTable), database_config_result);
    }

    #[test]
    fn no_matching_connection_returns_error() {
        // Laboriously setup the config extras
        let mut database_extra = BTreeMap::new();
        let mut connection_config = BTreeMap::new();
        connection_config.insert("url".to_string(), Value::from("dummy_db.sqlite"));
        connection_config.insert("pool_size".to_string(), Value::from(10));
        database_extra.insert("dummy_db".to_string(), Value::from(connection_config));

        let config = Config::build(Environment::Development)
            .extra("databases", database_extra)
            .finalize()
            .unwrap();

        let database_config_result = database_config("real_db", &config);

        assert_eq!(Err(DatabaseConfigError::MissingKey), database_config_result);
    }

    #[test]
    fn incorrectly_structured_config_returns_error() {
        let mut database_extra = BTreeMap::new();
        let connection_config = vec!["url", "dummy_db.slqite"];
        database_extra.insert("dummy_db".to_string(), Value::from(connection_config));

        let config = Config::build(Environment::Development)
            .extra("databases", database_extra)
            .finalize()
            .unwrap();

        let database_config_result = database_config("dummy_db", &config);

        assert_eq!(Err(DatabaseConfigError::MalformedConfiguration), database_config_result);
    }

    #[test]
    fn missing_connection_string_returns_error() {
        let mut database_extra = BTreeMap::new();
        let connection_config: BTreeMap<String, Value> = BTreeMap::new();
        database_extra.insert("dummy_db", connection_config);

        let config = Config::build(Environment::Development)
            .extra("databases", database_extra)
            .finalize()
            .unwrap();

        let database_config_result = database_config("dummy_db", &config);

        assert_eq!(Err(DatabaseConfigError::MissingUrl), database_config_result);
    }

    #[test]
    fn invalid_connection_string_returns_error() {
        let mut database_extra = BTreeMap::new();
        let mut connection_config = BTreeMap::new();
        connection_config.insert("url".to_string(), Value::from(42));
        database_extra.insert("dummy_db", connection_config);

        let config = Config::build(Environment::Development)
            .extra("databases", database_extra)
            .finalize()
            .unwrap();

        let database_config_result = database_config("dummy_db", &config);

        assert_eq!(Err(DatabaseConfigError::MalformedUrl), database_config_result);
    }

    #[test]
    fn negative_pool_size_returns_error() {
        let mut database_extra = BTreeMap::new();
        let mut connection_config = BTreeMap::new();
        connection_config.insert("url".to_string(), Value::from("dummy_db.sqlite"));
        connection_config.insert("pool_size".to_string(), Value::from(-1));
        database_extra.insert("dummy_db", connection_config);

        let config = Config::build(Environment::Development)
            .extra("databases", database_extra)
            .finalize()
            .unwrap();

        let database_config_result = database_config("dummy_db", &config);

        assert_eq!(Err(DatabaseConfigError::InvalidPoolSize(-1)), database_config_result);
    }

    #[test]
    fn pool_size_beyond_u32_max_returns_error() {
        let mut database_extra = BTreeMap::new();
        let mut connection_config = BTreeMap::new();
        connection_config.insert("url".to_string(), Value::from("dummy_db.sqlite"));
        connection_config.insert("pool_size".to_string(), Value::from(4294967296));
        database_extra.insert("dummy_db", connection_config);

        let config = Config::build(Environment::Development)
            .extra("databases", database_extra)
            .finalize()
            .unwrap();

        let database_config_result = database_config("dummy_db", &config);

        // The size of `0` is an overflow wrap-around
        assert_eq!(Err(DatabaseConfigError::InvalidPoolSize(0)), database_config_result);
    }

    #[test]
    fn happy_path_database_config() {
        let url = "dummy_db.sqlite";
        let pool_size = 10;

        let mut database_extra = BTreeMap::new();
        let mut connection_config = BTreeMap::new();
        connection_config.insert("url".to_string(), Value::from(url));
        connection_config.insert("pool_size".to_string(), Value::from(pool_size));
        database_extra.insert("dummy_db", connection_config);

        let config = Config::build(Environment::Development)
            .extra("databases", database_extra)
            .finalize()
            .unwrap();

        let database_config = database_config("dummy_db", &config).unwrap();

        assert_eq!(url, database_config.url);
        assert_eq!(pool_size, database_config.pool_size);
        assert_eq!(0, database_config.extras.len());
    }

    #[test]
    fn extras_do_not_contain_required_keys() {
        let url = "dummy_db.sqlite";
        let pool_size = 10;

        let mut database_extra = BTreeMap::new();
        let mut connection_config = BTreeMap::new();
        connection_config.insert("url".to_string(), Value::from(url));
        connection_config.insert("pool_size".to_string(), Value::from(pool_size));
        database_extra.insert("dummy_db", connection_config);

        let config = Config::build(Environment::Development)
            .extra("databases", database_extra)
            .finalize()
            .unwrap();

        let database_config = database_config("dummy_db", &config).unwrap();

        assert_eq!(url, database_config.url);
        assert_eq!(pool_size, database_config.pool_size);
        assert_eq!(false, database_config.extras.contains_key("url"));
        assert_eq!(false, database_config.extras.contains_key("pool_size"));
    }

    #[test]
    fn extra_values_are_placed_in_extras_map() {
        let url = "dummy_db.sqlite";
        let pool_size = 10;
        let tls_cert = "certs.pem";
        let tls_key = "key.pem";

        let mut database_extra = BTreeMap::new();
        let mut connection_config = BTreeMap::new();
        connection_config.insert("url".to_string(), Value::from(url));
        connection_config.insert("pool_size".to_string(), Value::from(pool_size));
        connection_config.insert("certs".to_string(), Value::from(tls_cert));
        connection_config.insert("key".to_string(), Value::from(tls_key));
        database_extra.insert("dummy_db", connection_config);

        let config = Config::build(Environment::Development)
            .extra("databases", database_extra)
            .finalize()
            .unwrap();

        let database_config = database_config("dummy_db", &config).unwrap();

        assert_eq!(url, database_config.url);
        assert_eq!(pool_size, database_config.pool_size);
        assert_eq!(true, database_config.extras.contains_key("certs"));
        assert_eq!(true, database_config.extras.contains_key("key"));

        println!("{:#?}", database_config);
    }
}
