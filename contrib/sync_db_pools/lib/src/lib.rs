//! Traits, utilities, and a macro for easy database connection pooling.
//!
//! # Overview
//!
//! This crate provides traits, utilities, and a procedural macro for
//! configuring and accessing database connection pools in Rocket. A _database
//! connection pool_ is a data structure that maintains active database
//! connections for later use in the application. This implementation is backed
//! by [`r2d2`] and exposes connections through request guards.
//!
//! Databases are individually configured through Rocket's regular configuration
//! mechanisms. Connecting a Rocket application to a database using this library
//! occurs in three simple steps:
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
//! in `rocket_sync_db_pools` must be enabled:
//!
//! ```toml
//! [dependencies.rocket_sync_db_pools]
//! version = "0.1.0-rc.1"
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
//! [default.databases]
//! sqlite_logs = { url = "/path/to/database.sqlite" }
//! ```
//!
//! In your application's source code, one-time:
//!
//! ```rust
//! # #[macro_use] extern crate rocket;
//! # #[cfg(feature = "diesel_sqlite_pool")]
//! # mod test {
//! use rocket_sync_db_pools::{database, diesel};
//!
//! #[database("sqlite_logs")]
//! struct LogsDbConn(diesel::SqliteConnection);
//!
//! #[launch]
//! fn rocket() -> _ {
//!     rocket::build().attach(LogsDbConn::fairing())
//! }
//! # } fn main() {}
//! ```
//!
//! Whenever a connection to the database is needed:
//!
//! ```rust
//! # #[macro_use] extern crate rocket;
//! # #[macro_use] extern crate rocket_sync_db_pools;
//! #
//! # #[cfg(feature = "diesel_sqlite_pool")]
//! # mod test {
//! # use rocket_sync_db_pools::diesel;
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
//! url = "postgres://root:root@localhost/my_db"
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
//!     pool (defaults to the configured number of workers * 4)
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
//! # use rocket::launch;
//! use rocket::figment::{value::{Map, Value}, util::map};
//!
//! #[launch]
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
//! guide](https://rocket.rs/v0.5-rc/guide/configuration/#environment-variables):
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
//! attribute accepts a single string parameter that indicates the name of the
//! database. This corresponds to the database name set as the database's
//! configuration key.
//!
//! The macro generates a [`FromRequest`] implementation for the decorated type,
//! allowing the type to be used as a request guard. This implementation
//! retrieves a connection from the database pool or fails with a
//! `Status::ServiceUnavailable` if connecting to the database times out.
//!
//! The macro also generates three inherent methods on the decorated type:
//!
//!   * `fn fairing() -> impl Fairing`
//!
//!      Returns a fairing that initializes the associated database connection
//!      pool.
//!
//!   * `async fn get_one<P: Phase>(&Rocket<P>) -> Option<Self>`
//!
//!     Retrieves a connection wrapper from the configured pool. Returns `Some`
//!     as long as `Self::fairing()` has been attached.
//!
//!   * `async fn run<R: Send + 'static>(&self, impl FnOnce(&mut Db) -> R + Send + 'static) -> R`
//!
//!     Runs the specified function or closure, providing it access to the
//!     underlying database connection (`&mut Db`). Returns the value returned
//!     by the function or closure.
//!
//! The attribute can only be applied to unit-like structs with one type. The
//! internal type of the structure must implement [`Poolable`].
//!
//! ```rust
//! # #[macro_use] extern crate rocket_sync_db_pools;
//! # #[cfg(feature = "diesel_sqlite_pool")]
//! # mod test {
//! use rocket_sync_db_pools::diesel;
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
//! # #[macro_use] extern crate rocket_sync_db_pools;
//! # #[cfg(feature = "postgres_pool")]
//! # mod test {
//! use rocket_sync_db_pools::postgres;
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
//! # #[macro_use] extern crate rocket_sync_db_pools;
//! #
//! # #[cfg(feature = "diesel_sqlite_pool")] {
//! # use rocket::figment::{value::{Map, Value}, util::map};
//! use rocket_sync_db_pools::diesel;
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
//! # #[macro_use] extern crate rocket_sync_db_pools;
//! #
//! # #[cfg(feature = "diesel_sqlite_pool")]
//! # mod test {
//! # use rocket_sync_db_pools::diesel;
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
//! # #[macro_use] extern crate rocket_sync_db_pools;
//! #
//! # #[cfg(feature = "diesel_sqlite_pool")]
//! # mod test {
//! # use rocket_sync_db_pools::diesel;
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
//! | Postgres | [Diesel]              | `1`       | [`diesel::PgConnection`]       | `diesel_postgres_pool` |
//! | Postgres | [Rust-Postgres]       | `0.19`    | [`postgres::Client`]           | `postgres_pool`        |
//! | Sqlite   | [Diesel]              | `1`       | [`diesel::SqliteConnection`]   | `diesel_sqlite_pool`   |
//! | Sqlite   | [`Rusqlite`]          | `0.24`    | [`rusqlite::Connection`]       | `sqlite_pool`          |
//! | Memcache | [`memcache`]          | `0.15`    | [`memcache::Client`]           | `memcache_pool`        |
//!
//! [Diesel]: https://diesel.rs
//! [`rusqlite::Connection`]: https://docs.rs/rusqlite/0.23.0/rusqlite/struct.Connection.html
//! [`diesel::SqliteConnection`]: http://docs.diesel.rs/diesel/prelude/struct.SqliteConnection.html
//! [`postgres::Client`]: https://docs.rs/postgres/0.19/postgres/struct.Client.html
//! [`diesel::PgConnection`]: http://docs.diesel.rs/diesel/pg/struct.PgConnection.html
//! [`diesel::MysqlConnection`]: http://docs.diesel.rs/diesel/mysql/struct.MysqlConnection.html
//! [`Rusqlite`]: https://github.com/jgallagher/rusqlite
//! [Rust-Postgres]: https://github.com/sfackler/rust-postgres
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
//! [`Poolable`]: crate::Poolable

#![doc(html_root_url = "https://api.rocket.rs/v0.5-rc/rocket_sync_db_pools")]
#![doc(html_favicon_url = "https://rocket.rs/images/favicon.ico")]
#![doc(html_logo_url = "https://rocket.rs/images/logo-boxed.png")]

#[doc(hidden)]
#[macro_use]
pub extern crate rocket;

#[cfg(any(
    feature = "diesel_sqlite_pool",
    feature = "diesel_postgres_pool",
    feature = "diesel_mysql_pool"
))]
pub use diesel;

#[cfg(feature = "postgres_pool")] pub use postgres;
#[cfg(feature = "postgres_pool")] pub use r2d2_postgres;

#[cfg(feature = "sqlite_pool")] pub use rusqlite;
#[cfg(feature = "sqlite_pool")] pub use r2d2_sqlite;

#[cfg(feature = "memcache_pool")] pub use memcache;
#[cfg(feature = "memcache_pool")] pub use r2d2_memcache;

pub use r2d2;

mod poolable;
mod config;
mod error;
mod connection;

pub use self::poolable::{Poolable, PoolResult};
pub use self::config::Config;
pub use self::error::Error;

pub use rocket_sync_db_pools_codegen::*;
pub use self::connection::*;
