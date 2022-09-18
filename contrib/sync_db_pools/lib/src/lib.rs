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
//! version = "0.1.0-rc.2"
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
//! To configure a database via `Rocket.toml`, add a table for each database to
//! the `databases` table where the key is a name of your choice. The table
//! should have a `url` key and, optionally, `pool_size` and `timeout` keys.
//! This looks as follows:
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
//! # With `pool_size` and `timeout` keys:
//! [global.databases.sqlite_db]
//! url = "db.sqlite"
//! pool_size = 20
//! timeout = 5
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
//!   * `timeout` - max number of seconds to wait for a connection to become
//!     available (defaults to `5`)
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
//!         "pool_size" => 10.into(),
//!         "timeout" => 5.into(),
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
//! See [`ExampleDb`](example::ExampleDb) for everything that the macro
//! generates. Specifically, it generates:
//!
//!   * A [`FromRequest`] implementation for the decorated type.
//!   * A [`Sentinel`](rocket::Sentinel) implementation for the decorated type.
//!   * A [`fairing()`](example::ExampleDb::fairing()) method to initialize the
//!     database.
//!   * A [`run()`](example::ExampleDb::run()) method to execute blocking
//!     database operations in an `async`-safe manner.
//!   * A [`pool()`](example::ExampleDb::pool()) method to retrieve the
//!     backing connection pool.
//!
//! The attribute can only be applied to tuple structs with one field. The
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
//! | Sqlite   | [Diesel]              | `2`       | [`diesel::SqliteConnection`]   | `diesel_sqlite_pool`   |
//! | Postgres | [Diesel]              | `2`       | [`diesel::PgConnection`]       | `diesel_postgres_pool` |
//! | MySQL    | [Diesel]              | `2`       | [`diesel::MysqlConnection`]    | `diesel_mysql_pool`    |
//! | Postgres | [Rust-Postgres]       | `0.19`    | [`postgres::Client`]           | `postgres_pool`        |
//! | Sqlite   | [`Rusqlite`]          | `0.27`    | [`rusqlite::Connection`]       | `sqlite_pool`          |
//! | Memcache | [`memcache`]          | `0.15`    | [`memcache::Client`]           | `memcache_pool`        |
//!
//! [Diesel]: https://diesel.rs
//! [`diesel::SqliteConnection`]: https://docs.rs/diesel/2/diesel/sqlite/struct.SqliteConnection.html
//! [`diesel::PgConnection`]: https://docs.rs/diesel/2/diesel/pg/struct.PgConnection.html
//! [`diesel::MysqlConnection`]: https://docs.rs/diesel/2/diesel/mysql/struct.MysqlConnection.html
//! [Rust-Postgres]: https://github.com/sfackler/rust-postgres
//! [`postgres::Client`]: https://docs.rs/postgres/0.19/postgres/struct.Client.html
//! [`Rusqlite`]: https://github.com/jgallagher/rusqlite
//! [`rusqlite::Connection`]: https://docs.rs/rusqlite/0.27/rusqlite/struct.Connection.html
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
#![cfg_attr(nightly, feature(doc_cfg))]

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

/// Example of code generated by the `#[database]` attribute.
#[cfg(all(nightly, doc, feature = "diesel_sqlite_pool"))]
pub mod example {
    use crate::diesel;

    /// Example of code generated by the `#[database]` attribute.
    ///
    /// This implementation of `ExampleDb` was generated by:
    ///
    /// ```rust
    /// use rocket_sync_db_pools::{database, diesel};
    ///
    /// #[database("example")]
    /// pub struct ExampleDb(diesel::SqliteConnection);
    /// ```
    pub struct ExampleDb(crate::Connection<Self, diesel::SqliteConnection>);

    impl ExampleDb {
        /// Returns a fairing that initializes the database connection pool
        /// associated with `Self`.
        ///
        /// The fairing _must_ be attached before `Self` can be used as a
        /// request guard.
        ///
        /// # Example
        ///
        /// ```rust
        /// # #[macro_use] extern crate rocket;
        /// # #[macro_use] extern crate rocket_sync_db_pools;
        /// #
        /// # #[cfg(feature = "diesel_sqlite_pool")] {
        /// use rocket_sync_db_pools::diesel;
        ///
        /// #[database("my_db")]
        /// struct MyConn(diesel::SqliteConnection);
        ///
        /// #[launch]
        /// fn rocket() -> _ {
        ///     rocket::build().attach(MyConn::fairing())
        /// }
        /// # }
        /// ```
        pub fn fairing() -> impl crate::rocket::fairing::Fairing {
            <crate::ConnectionPool<Self, diesel::SqliteConnection>>::fairing(
                "'example' Database Pool",
                "example",
            )
        }

        /// Returns an opaque type that represents the connection pool backing
        /// connections of type `Self` _as long as_ the fairing returned by
        /// [`Self::fairing()`] is attached and has run on `__rocket`.
        ///
        /// The returned pool is `Clone`. Values of type `Self` can be retrieved
        /// from the pool by calling `pool.get().await` which has the same
        /// signature and semantics as [`Self::get_one()`].
        ///
        /// # Example
        ///
        /// ```rust
        /// # #[macro_use] extern crate rocket;
        /// # #[macro_use] extern crate rocket_sync_db_pools;
        /// #
        /// # #[cfg(feature = "diesel_sqlite_pool")] {
        /// use rocket::tokio::{task, time};
        /// use rocket::fairing::AdHoc;
        /// use rocket_sync_db_pools::diesel;
        ///
        /// #[database("my_db")]
        /// struct MyConn(diesel::SqliteConnection);
        ///
        /// #[launch]
        /// fn rocket() -> _ {
        ///     rocket::build()
        ///         .attach(MyConn::fairing())
        ///         .attach(AdHoc::try_on_ignite("Background DB", |rocket| async {
        ///             let pool = match MyConn::pool(&rocket) {
        ///                 Some(pool) => pool.clone(),
        ///                 None => return Err(rocket)
        ///             };
        ///
        ///             // Start a background task that runs some database
        ///             // operation every 10 seconds. If a connection isn't
        ///             // available, retries 10 + timeout seconds later.
        ///             tokio::task::spawn(async move {
        ///                 loop {
        ///                     time::sleep(time::Duration::from_secs(10)).await;
        ///                     if let Some(conn) = pool.get().await {
        ///                         conn.run(|c| { /* perform db ops */ }).await;
        ///                     }
        ///                 }
        ///             });
        ///
        ///             Ok(rocket)
        ///         }))
        /// }
        /// # }
        /// ```
        pub fn pool<P: crate::rocket::Phase>(
            __rocket: &crate::rocket::Rocket<P>,
        ) -> Option<&crate::ConnectionPool<Self, diesel::SqliteConnection>>
        {
            <crate::ConnectionPool<Self, diesel::SqliteConnection>>::pool(
                &__rocket,
            )
        }

        /// Runs the provided function `__f` in an async-safe blocking thread.
        /// The function is supplied with a mutable reference to the raw
        /// connection (a value of type `&mut Self.0`). `.await`ing the return
        /// value of this function yields the value returned by `__f`.
        ///
        /// # Example
        ///
        /// ```rust
        /// # #[macro_use] extern crate rocket;
        /// # #[macro_use] extern crate rocket_sync_db_pools;
        /// #
        /// # #[cfg(feature = "diesel_sqlite_pool")] {
        /// use rocket_sync_db_pools::diesel;
        ///
        /// #[database("my_db")]
        /// struct MyConn(diesel::SqliteConnection);
        ///
        /// #[get("/")]
        /// async fn f(conn: MyConn) {
        ///     // The type annotation is illustrative and isn't required.
        ///     let result = conn.run(|c: &mut diesel::SqliteConnection| {
        ///         // Use `c`.
        ///     }).await;
        /// }
        /// # }
        /// ```
        pub async fn run<F, R>(&self, __f: F) -> R
        where
            F: FnOnce(&mut diesel::SqliteConnection) -> R + Send + 'static,
            R: Send + 'static,
        {
            self.0.run(__f).await
        }

        /// Retrieves a connection of type `Self` from the `rocket` instance.
        /// Returns `Some` as long as `Self::fairing()` has been attached and
        /// there is a connection available within at most `timeout` seconds.
        pub async fn get_one<P: crate::rocket::Phase>(
            __rocket: &crate::rocket::Rocket<P>,
        ) -> Option<Self> {
            <crate::ConnectionPool<Self, diesel::SqliteConnection>>::get_one(
                &__rocket,
            )
            .await
            .map(Self)
        }
    }

    /// Retrieves a connection from the database pool or fails with a
    /// `Status::ServiceUnavailable` if doing so times out.
    impl<'r> crate::rocket::request::FromRequest<'r> for ExampleDb {
        type Error = ();
        #[allow(
            clippy::let_unit_value,
            clippy::no_effect_underscore_binding,
            clippy::shadow_same,
            clippy::type_complexity,
            clippy::type_repetition_in_bounds,
            clippy::used_underscore_binding
        )]
        fn from_request<'life0, 'async_trait>(
            __r: &'r crate::rocket::request::Request<'life0>,
        ) -> ::core::pin::Pin<
            Box<
                dyn ::core::future::Future<
                        Output = crate::rocket::request::Outcome<Self, ()>,
                    > + ::core::marker::Send
                    + 'async_trait,
            >,
        >
        where
            'r: 'async_trait,
            'life0: 'async_trait,
            Self: 'async_trait,
        {
            Box::pin(async move {
                if let ::core::option::Option::Some(__ret) = ::core::option::Option::None::<
                    crate::rocket::request::Outcome<Self, ()>,
                > {
                    return __ret;
                }
                let __r = __r;
                let __ret: crate::rocket::request::Outcome<Self, ()> = {
                    < crate :: Connection < Self , diesel :: SqliteConnection > >
                        :: from_request (__r) . await . map (Self)
                };
                #[allow(unreachable_code)]
                __ret
            })
        }
    }
    impl crate::rocket::Sentinel for ExampleDb {
        fn abort(
            __r: &crate::rocket::Rocket<crate::rocket::Ignite>,
        ) -> bool {
            <crate::Connection<Self, diesel::SqliteConnection>>::abort(__r)
        }
    }
}
