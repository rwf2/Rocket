//! Asynchronous database driver connection pooling integration for Rocket.
//!
//! # Quickstart
//!
//! 1. Add `rocket_db_pools` as a dependency with one or more [database driver
//!    features](#supported-drivers) enabled:
//!
//!    ```toml
//!    [dependencies.rocket_db_pools]
//!    version = "0.1.0-rc.2"
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
//!            .fetch_one(&mut *db).await
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
//! At present, this crate supports _three_ drivers: [`deadpool`], [`sqlx`], and
//! [`mongodb`]. Each driver may support multiple databases. Drivers have a
//! varying degree of support for graceful shutdown, affected by the
//! `Type::init()` fairing on Rocket shutdown.
//!
//! ## `deadpool` (v0.9)
//!
//! | Database | Feature             | [`Pool`] Type               | [`Connection`] Deref                  |
//! |----------|---------------------|-----------------------------|---------------------------------------|
//! | Postgres | `deadpool_postgres` | [`deadpool_postgres::Pool`] | [`deadpool_postgres::ClientWrapper`]  |
//! | Redis    | `deadpool_redis`    | [`deadpool_redis::Pool`]    | [`deadpool_redis::Connection`]        |
//!
//! On shutdown, new connections are denied. Shutdown _does not_ wait for
//! connections to be returned.
//!
//! ## `sqlx` (v0.6)
//!
//! | Database | Feature         | [`Pool`] Type        | [`Connection`] Deref                     |
//! |----------|-----------------|----------------------|------------------------------------------|
//! | Postgres | `sqlx_postgres` | [`sqlx::PgPool`]     | [`sqlx::pool::PoolConnection<Postgres>`] |
//! | MySQL    | `sqlx_mysql`    | [`sqlx::MySqlPool`]  | [`sqlx::pool::PoolConnection<MySql>`]    |
//! | SQLite   | `sqlx_sqlite`   | [`sqlx::SqlitePool`] | [`sqlx::pool::PoolConnection<Sqlite>`]   |
//! | MSSQL    | `sqlx_mssql`    | [`sqlx::MssqlPool`]  | [`sqlx::pool::PoolConnection<Mssql>`]    |
//!
//! [`sqlx::PgPool`]: https://docs.rs/sqlx/0.6/sqlx/type.PgPool.html
//! [`sqlx::MySqlPool`]: https://docs.rs/sqlx/0.6/sqlx/type.MySqlPool.html
//! [`sqlx::SqlitePool`]: https://docs.rs/sqlx/0.6/sqlx/type.SqlitePool.html
//! [`sqlx::MssqlPool`]: https://docs.rs/sqlx/0.6/sqlx/type.MssqlPool.html
//! [`sqlx::PoolConnection<Postgres>`]: https://docs.rs/sqlx/0.6/sqlx/pool/struct.PoolConnection.html
//! [`sqlx::PoolConnection<MySql>`]: https://docs.rs/sqlx/0.6/sqlx/pool/struct.PoolConnection.html
//! [`sqlx::PoolConnection<Sqlite>`]: https://docs.rs/sqlx/0.6/sqlx/pool/struct.PoolConnection.html
//! [`sqlx::PoolConnection<Mssql>`]: https://docs.rs/sqlx/0.6/sqlx/pool/struct.PoolConnection.html
//!
//! On shutdown, new connections are denied. Shutdown waits for connections to
//! be returned.
//!
//! ## `mongodb` (v2)
//!
//! | Database | Feature   | [`Pool`] Type and [`Connection`] Deref |
//! |----------|-----------|----------------------------------------|
//! | MongoDB  | `mongodb` | [`mongodb::Client`]                    |
//!
//! Graceful shutdown is not supported.
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
//! version = "0.5"
//! default-features = false
//! features = ["macros", "offline", "migrate"]
//!
//! [dependencies.rocket_db_pools]
//! version = "0.1.0-rc.2"
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
//! # only `url` is required. the rest have defaults and are thus optional
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
//! **Note:** `deadpool` drivers do not support and thus ignore the
//! `min_connections` value.
//!
//! ## Driver Defaults
//!
//! Some drivers provide configuration defaults different from the underyling
//! database's defaults. A best-effort attempt is made to document those
//! differences below:
//!
//! * `sqlx_sqlite`
//!
//!   - foreign keys   : `enabled`
//!   - journal mode   : `WAL`
//!   - create-missing :  `enabled`
//!   - synchronous    : `full` (even when `WAL`)
//!   - busy timeout   : `connection_timeout`
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
//! # Extending
//!
//! Any database driver can implement support for this libary by implementing
//! the [`Pool`] trait.

#![doc(html_root_url = "https://api.rocket.rs/master/rocket_db_pools")]
#![doc(html_favicon_url = "https://rocket.rs/images/favicon.ico")]
#![doc(html_logo_url = "https://rocket.rs/images/logo-boxed.png")]

#![deny(missing_docs)]

/// Re-export of the `figment` crate.
#[doc(inline)]
pub use rocket::figment;

pub use rocket;
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
pub use self::pool::Pool;
pub use self::config::Config;

pub use rocket_db_pools_codegen::*;
