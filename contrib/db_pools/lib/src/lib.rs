//! Asynchronous database driver connection pooling integration for Rocket.
//!
//! # Quickstart
//!
//! 1. Add `rocket_db_pools` as a dependency with one or more [database driver
//!    features](#supported-drivers) enabled:
//!
//!    ```toml
//!    [dependencies.rocket_db_pools]
//!    version = "0.1.0"
//!    features = ["sqlx_sqlite"]
//!    ```
//!
//! 2. Choose a name for your database, here `sqlite_logs`.
//!    [Configure](#configuration) _at least_ a URL for the database:
//!
//!    ```toml
//!    [default.databases.sqlite_logs]
//!    url = "/path/to/database.sqlite"
//!    ```
//!
//! 3. [Derive](derive@Database) [`Database`] for a unit type (`Logs` here)
//!    which wraps the selected driver's [`Pool`] type (see [the driver
//!    table](#supported-drivers)) and is decorated with `#[database("name")]`.
//!    Attach `Type::init()` to your application's `Rocket` to initialize the
//!    database pool:
//!
//!    ```rust
//!    # #[cfg(feature = "sqlx_sqlite")] mod _inner {
//!    # use rocket::launch;
//!    use rocket_db_pools::{sqlx, Database};
//!
//!    #[derive(Database)]
//!    #[database("sqlite_logs")]
//!    struct Logs(sqlx::SqlitePool);
//!
//!    #[launch]
//!    fn rocket() -> _ {
//!        rocket::build().attach(Logs::init())
//!    }
//!    # }
//!    ```
//!
//! 4. Use [`Connection<Type>`](Connection) as a request guard to retrieve an
//!    active database connection, which dereferences to the native type in the
//!    [`Connection` deref](#supported-drivers) column.
//!
//!    ```rust
//!    # #[cfg(feature = "sqlx_sqlite")] mod _inner {
//!    # use rocket::{get, response::Responder};
//!    # use rocket_db_pools::{sqlx, Database};
//!    # #[derive(Database)]
//!    # #[database("sqlite_logs")]
//!    # struct Logs(sqlx::SqlitePool);
//!    #
//!    # #[derive(Responder)]
//!    # struct Log(String);
//!    #
//!    use rocket_db_pools::Connection;
//!    use rocket_db_pools::sqlx::Row;
//!
//!    #[get("/<id>")]
//!    async fn read(mut db: Connection<Logs>, id: i64) -> Option<Log> {
//!        sqlx::query("SELECT content FROM logs WHERE id = ?").bind(id)
//!            .fetch_one(&mut **db).await
//!            .and_then(|r| Ok(Log(r.try_get(0)?)))
//!            .ok()
//!    }
//!    # }
//!    ```
//!
//!    Alternatively, use a reference to the database type as a request guard to
//!    retrieve the entire pool, but note that unlike retrieving a `Connection`,
//!    doing so does _not_ guarantee that a connection is available:
//!
//!    ```rust
//!    # #[cfg(feature = "sqlx_sqlite")] mod _inner {
//!    # use rocket::{get, response::Responder};
//!    # use rocket_db_pools::{sqlx, Database};
//!    # #[derive(Database)]
//!    # #[database("sqlite_logs")]
//!    # struct Logs(sqlx::SqlitePool);
//!    #
//!    # #[derive(Responder)]
//!    # struct Log(String);
//!    #
//!    use rocket_db_pools::sqlx::Row;
//!
//!    #[get("/<id>")]
//!    async fn read(db: &Logs, id: i64) -> Option<Log> {
//!        sqlx::query("SELECT content FROM logs WHERE id = ?").bind(id)
//!            .fetch_one(&db.0).await
//!            .and_then(|r| Ok(Log(r.try_get(0)?)))
//!            .ok()
//!    }
//!    # }
//!    ```
//!
//! # Supported Drivers
//!
//! At present, this crate supports _four_ drivers: [`deadpool`], [`sqlx`],
//! [`mongodb`], and [`diesel`]. Each driver may support multiple databases.
//! Drivers have a varying degree of support for graceful shutdown, affected by
//! the `Type::init()` fairing on Rocket shutdown.
//!
//! ## `deadpool` (v0.13)
//!
//! | Database | Feature                     | [`Pool`] Type               | [`Connection`] Deref                 |
//! |----------|-----------------------------|-----------------------------|--------------------------------------|
//! | Postgres | `deadpool_postgres` (v0.14) | [`deadpool_postgres::Pool`] | [`deadpool_postgres::ClientWrapper`] |
//! | Redis    | `deadpool_redis` (v0.23)    | [`deadpool_redis::Pool`]    | [`deadpool_redis::Connection`]       |
//!
//! On shutdown, new connections are denied. Shutdown _does not_ wait for
//! connections to be returned.
//!
//! See [Postgres TLS](#postgres-tls) to connect to Postgres over TLS.
//!
//! ## `sqlx` (v0.9)
//!
//! | Database | Feature         | [`Pool`] Type        | [`Connection`] Deref                     |
//! |----------|-----------------|----------------------|------------------------------------------|
//! | Postgres | `sqlx_postgres` | [`sqlx::PgPool`]     | [`sqlx::pool::PoolConnection<Postgres>`] |
//! | MySQL    | `sqlx_mysql`    | [`sqlx::MySqlPool`]  | [`sqlx::pool::PoolConnection<MySql>`]    |
//! | SQLite   | `sqlx_sqlite`   | [`sqlx::SqlitePool`] | [`sqlx::pool::PoolConnection<Sqlite>`]   |
//!
//! [`sqlx::PgPool`]: https://docs.rs/sqlx/0.9/sqlx/type.PgPool.html
//! [`sqlx::MySqlPool`]: https://docs.rs/sqlx/0.9/sqlx/type.MySqlPool.html
//! [`sqlx::SqlitePool`]: https://docs.rs/sqlx/0.9/sqlx/type.SqlitePool.html
//! [`sqlx::pool::PoolConnection<Postgres>`]: https://docs.rs/sqlx/0.9/sqlx/pool/struct.PoolConnection.html
//! [`sqlx::pool::PoolConnection<MySql>`]: https://docs.rs/sqlx/0.9/sqlx/pool/struct.PoolConnection.html
//! [`sqlx::pool::PoolConnection<Sqlite>`]: https://docs.rs/sqlx/0.9/sqlx/pool/struct.PoolConnection.html
//!
//! On shutdown, new connections are denied. Shutdown waits for connections to
//! be returned.
//!
//! ## `mongodb` (v3)
//!
//! | Database | Feature   | [`Pool`] Type and [`Connection`] Deref |
//! |----------|-----------|----------------------------------------|
//! | MongoDB  | `mongodb` | [`mongodb::Client`]                    |
//!
//! Graceful shutdown is not supported.
//!
//! ## `diesel` (v2)
//!
//! | Database | Feature           | [`Pool`] Type          | [`Connection`] Deref                                        |
//! |----------|-------------------|------------------------|-------------------------------------------------------------|
//! | Postgres | `diesel_postgres` | [`diesel::PgPool`]     | [`diesel::AsyncPgConnection`]                               |
//! | MySQL    | `diesel_mysql`    | [`diesel::MysqlPool`]  | [`diesel::AsyncMysqlConnection`]                            |
//! | SQLite   | `diesel_sqlite`   | [`diesel::SqlitePool`] | [`diesel::SyncConnectionWrapper<diesel::SqliteConnection>`] |
//!
//! See [`diesel`] for usage details.
//!
//! On shutdown, new connections are denied. Shutdown _does not_ wait for
//! connections to be returned.
//!
//! See [Postgres TLS](#postgres-tls) to connect to Postgres over TLS.
//!
//! ## Postgres TLS
//!
//! By default, the `deadpool_postgres` and `diesel_postgres` drivers connect
//! _without_ TLS: a URL with `sslmode=require` fails to connect. To enable TLS
//! for these drivers, also enable the `postgres_rustls_native_certs` feature:
//!
//! ```toml
//! [dependencies.rocket_db_pools]
//! version = "0.1.0"
//! features = ["diesel_postgres", "postgres_rustls_native_certs"]
//! ```
//!
//! To require TLS, set `sslmode=require` in the database URL. With
//! `sslmode=prefer`, a connection without TLS is allowed if the server does not
//! support TLS. The server's certificate and hostname are verified using the
//! platform's native root certificates.
//!
//! To trust a private CA, set `SSL_CERT_FILE` to a PEM certificate bundle or
//! `SSL_CERT_DIR` to a directory of PEM certificates. These replace the native
//! roots. Errors loading certificates are logged. If no root certificates can
//! be loaded, TLS connections fail.
//!
//! The `sqlx_postgres` driver supports TLS without additional features.
//!
//! ## Enabling Additional Driver Features
//!
//! Only the minimal features for each driver crate are enabled by
//! `rocket_db_pools`. To use additional driver functionality exposed via its
//! crate's features, you'll need to depend on the crate directly with those
//! features enabled in `Cargo.toml`:
//!
//! ```toml
//! [dependencies.sqlx]
//! version = "0.9"
//! default-features = false
//! features = ["macros", "migrate"]
//!
//! [dependencies.rocket_db_pools]
//! version = "0.1.0"
//! features = ["sqlx_sqlite"]
//! ```
//!
//! # Configuration
//!
//! Configuration for a database named `db_name` is deserialized from a
//! `databases.db_name` configuration parameter into a [`Config`] structure via
//! Rocket's [configuration facilities](rocket::config). By default,
//! configuration can be provided in `Rocket.toml`:
//!
//! ```toml
//! [default.databases.db_name]
//! url = "db.sqlite"
//!
//! # Only `url` is required. These have sane defaults and are optional.
//! min_connections = 64
//! max_connections = 1024
//! connect_timeout = 5
//! idle_timeout = 120
//! ```
//!
//! Or via environment variables:
//!
//! ```sh
//! ROCKET_DATABASES='{db_name={url="db.sqlite",idle_timeout=120}}'
//! ```
//!
//! See [`Config`] for details on configuration parameters.
//!
//! **Note:** `deadpool` and `diesel` drivers do not support and thus ignore the
//! `min_connections` value. For these drivers, `idle_timeout` limits the time
//! allowed to recycle a connection before reuse, including any health check.
//! It does not close idle connections. By default, no timeout is set.
//!
//! ## Redis
//!
//! The `deadpool_redis` driver accepts an additional `response_timeout` option:
//!
//! ```toml
//! [default.databases.redis]
//! url = "redis://localhost:6379"
//! response_timeout = 0.5
//! ```
//!
//! This option limits how long the client waits for a command's response, in
//! seconds, and defaults to `0.5`. Both integer and fractional values are
//! accepted. Set `response_timeout = 0` to disable the timeout, for example,
//! when using blocking commands such as `BLPOP`. A timeout does not cancel a
//! command already executing on Redis. Negative, nonfinite, out-of-range, and
//! nonnumeric values result in an [`Error::Config`] during initialization.
//! Other drivers ignore this option.
//!
//! The Redis driver's connection timeout is disabled. Instead, Deadpool limits
//! both the time spent waiting for a pooled connection and the time spent
//! creating a connection to [`Config::connect_timeout`] seconds each. These
//! limits apply separately. Redis health checks also use `response_timeout`,
//! even when `idle_timeout` is unset.
//!
//! ## Driver Defaults
//!
//! Some drivers provide configuration defaults different from the underlying
//! database's defaults. A best-effort attempt is made to document those
//! differences below:
//!
//! * `deadpool_redis`
//!
//!   - response timeout   : `0.5` seconds
//!   - connection timeout : disabled (Deadpool's creation timeout applies)
//!
//! * `sqlx_sqlite`
//!
//!   - foreign keys   : `enabled`
//!   - journal mode   : `WAL`
//!   - create-missing :  `enabled`
//!   - synchronous    : `full` (even when `WAL`)
//!   - busy timeout   : `connect_timeout`
//!
//! * `sqlx_postgres`
//!
//!   - sslmode                  : `prefer`
//!   - statement-cache-capacity : `100`
//!   - user                     : result of `whoami`
//!
//! * `sqlx_mysql`
//!
//!   - sslmode                  : `PREFERRED`
//!   - statement-cache-capacity : `100`
//!
//! ## Customization
//!
//! To customize a database driver before its pool is created, use
//! [`Database::custom()`]. The pool must implement [`Configurable`], as SQLx
//! pools and MongoDB clients do. This method takes a function that receives a
//! reference to the database's [`Config`] and the driver's configuration
//! options, with Rocket's configuration and defaults already applied. The
//! returned options are used to create the pool.
//!
//! For SQLx, the options are a tuple of pool and connection options. The pool
//! options allow further customization via methods such as `after_connect()`.
//! For MongoDB, the options are a `ClientOptions` structure. See
//! [`Configurable::Options`] for the configuration types used by each driver.
//! As an example, the following sets MongoDB's application name:
//!
//! ```rust
//! # #[cfg(feature = "mongodb")] mod _inner {
//! use rocket_db_pools::{mongodb, Database};
//!
//! #[derive(Database)]
//! #[database("mongo_db")]
//! struct Db(mongodb::Client);
//!
//! fn main() {
//!     rocket::build().attach(Db::custom(|_config, mut options| {
//!         options.app_name = Some("my-app".into());
//!         options
//!     }));
//! }
//! # }
//! ```
//!
//! If configuration can fail, use [`Database::try_custom()`] instead. Returning
//! an error from the callback aborts launch.
//!
//! ### SQLite Extensions
//!
//! SQLite extensions can be loaded via SQLx's
//! `SqliteConnectOptions::extension()` method in a [`Database::custom()`]
//! callback. The method is `unsafe`: the extension library must be trusted
//! whenever a connection is created, including connections created after
//! initialization. For example, to load the `rot13` extension:
//!
//! ```rust
//! # #[cfg(feature = "sqlx_sqlite")] mod _inner {
//! use rocket_db_pools::{sqlx, Database};
//!
//! #[derive(Database)]
//! #[database("sqlite_db")]
//! struct Db(sqlx::SqlitePool);
//!
//! fn main() {
//!     rocket::build().attach(Db::custom(|_config, (pool, connection)| {
//!         // SAFETY: this library is trusted for the pool's lifetime.
//!         let connection = unsafe { connection.extension("rot13") };
//!         (pool, connection)
//!     }));
//! }
//! # }
//! ```
//!
//! To use extension names from application configuration, capture the
//! configuration in the callback.
//!
//! # Extending
//!
//! Any database driver can implement support for this library by implementing
//! the [`Pool`] trait. To support customization via [`Database::custom()`],
//! also implement [`Configurable`].

#![doc(html_root_url = "https://api.rocket.rs/master/rocket_db_pools")]
#![doc(html_favicon_url = "https://rocket.rs/images/favicon.ico")]
#![doc(html_logo_url = "https://rocket.rs/images/logo-boxed.png")]

#![deny(missing_docs)]

pub use rocket;

/// Re-export of the `figment` crate.
#[doc(inline)]
pub use rocket::figment;

#[cfg(any(
    feature = "diesel_postgres",
    feature = "diesel_mysql",
    feature = "diesel_sqlite"
))] pub mod diesel;
#[cfg(feature = "deadpool_postgres")] pub use deadpool_postgres;
#[cfg(feature = "deadpool_redis")] pub use deadpool_redis;
#[cfg(feature = "mongodb")] pub use mongodb;
#[cfg(feature = "sqlx")] pub use sqlx;

mod database;
mod error;
mod pool;
mod config;

pub use self::database::{Connection, Database, Initializer};
pub use self::error::Error;
pub use self::pool::{Pool, Configurable};
pub use self::config::Config;

pub use rocket_db_pools_codegen::*;
