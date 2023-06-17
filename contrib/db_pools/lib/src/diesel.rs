//! Re-export of [`diesel`] with prelude types overridden with `async` variants
//! from [`diesel_async`].
//!
//! # Usage
//!
//! To use `async` `diesel` support provided here, enable the following
//! dependencies in your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! rocket = "=0.5.0-rc.3"
//! diesel = "2"
//!
//! [dependencies.rocket_db_pools]
//! version = "=0.1.0-rc.3"
//! features = ["diesel_mysql"]
//! ```
//!
//! Then, import `rocket_db_pools::diesel::prelude::*` as well as the
//! appropriate pool type and, optionally, [`QueryResult`]. To use macros or
//! `diesel` functions, use `diesel::` directly. That is, _do not_ import
//! `rocket_db_pools::diesel`. Doing so will, by design, cause import errors.
//!
//! # Example
//!
//! ```rust
//! # #[macro_use] extern crate rocket;
//! # #[cfg(feature = "diesel_mysql")] {
//! use rocket_db_pools::{Database, Connection};
//! use rocket_db_pools::diesel::{QueryResult, MysqlPool, prelude::*};
//!
//! #[derive(Database)]
//! #[database("diesel_mysql")]
//! struct Db(MysqlPool);
//!
//! #[derive(Queryable, Insertable)]
//! #[diesel(table_name = posts)]
//! struct Post {
//!     id: i64,
//!     title: String,
//!     published: bool,
//! }
//!
//! diesel::table! {
//!     posts (id) {
//!         id -> BigInt,
//!         title -> Text,
//!         published -> Bool,
//!     }
//! }
//!
//! #[get("/")]
//! async fn list(mut db: Connection<Db>) -> QueryResult<String> {
//!     let post_ids: Vec<i64> = posts::table
//!         .select(posts::id)
//!         .load(&mut db)
//!         .await?;
//!
//!     Ok(format!("{post_ids:?}"))
//! }
//! # }
//! ```

/// The [`diesel`] prelude with `sync`-only traits replaced with their
/// [`diesel_async`] variants.
pub mod prelude {
    #[doc(inline)]
    pub use diesel::prelude::*;

    #[doc(inline)]
    pub use diesel_async::{AsyncConnection, RunQueryDsl, SaveChangesDsl};
}

#[doc(hidden)]
pub use diesel::*;

#[doc(hidden)]
pub use diesel_async::{RunQueryDsl, SaveChangesDsl, *};

#[doc(hidden)]
#[cfg(feature = "diesel_postgres")]
pub use diesel_async::pg;

#[doc(inline)]
pub use diesel_async::pooled_connection::deadpool::Pool;

#[doc(inline)]
#[cfg(feature = "diesel_mysql")]
pub use diesel_async::AsyncMysqlConnection;

#[doc(inline)]
#[cfg(feature = "diesel_postgres")]
pub use diesel_async::AsyncPgConnection;

/// Alias of a `Result` with an error type of [`Debug`] for a `diesel::Error`.
///
/// `QueryResult` is a [`Responder`](rocket::response::Responder) when `T` (the
/// `Ok` value) is a `Responder`. By using this alias as a route handler's
/// return type, the `?` operator can be applied to fallible `diesel` functions
/// in the route handler while still providing a valid `Responder` return type.
///
/// See the [module level docs](self#example) for a usage example.
///
/// [`Debug`]: rocket::response::Debug
pub type QueryResult<T, E = rocket::response::Debug<diesel::result::Error>> = Result<T, E>;

/// Type alias for an `async` pool of MySQL connections for `async` [diesel].
///
/// ```rust
/// # extern crate rocket;
/// # #[cfg(feature = "diesel_mysql")] {
/// # use rocket::get;
/// use rocket_db_pools::{Database, Connection};
/// use rocket_db_pools::diesel::{MysqlPool, prelude::*};
///
/// #[derive(Database)]
/// #[database("my_mysql_db_name")]
/// struct Db(MysqlPool);
///
/// #[get("/")]
/// async fn use_db(mut db: Connection<Db>) {
///     /* .. */
/// }
/// # }
/// ```
#[cfg(feature = "diesel_mysql")]
pub type MysqlPool = Pool<AsyncMysqlConnection>;

/// Type alias for an `async` pool of Postgres connections for `async` [diesel].
///
/// ```rust
/// # extern crate rocket;
/// # #[cfg(feature = "diesel_postgres")] {
/// # use rocket::get;
/// use rocket_db_pools::{Database, Connection};
/// use rocket_db_pools::diesel::{PgPool, prelude::*};
///
/// #[derive(Database)]
/// #[database("my_pg_db_name")]
/// struct Db(PgPool);
///
/// #[get("/")]
/// async fn use_db(mut db: Connection<Db>) {
///     /* .. */
/// }
/// # }
/// ```
#[cfg(feature = "diesel_postgres")]
pub type PgPool = Pool<AsyncPgConnection>;
