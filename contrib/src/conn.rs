use std::ops::Deref;

use r2d2::{self, ManageConnection, Pool, PooledConnection};

use rocket::http::Status;
use rocket::request::{self, FromRequest};
use rocket::{Outcome, Request, State};

/// The `Conn` type: implements `FromRequest`, allowing you to
/// easily access database from a connection pool. You can have multiple connection pool to multiple database backend.
///
/// ## Database support
/// This helper is using [r2d2](https://crates.io/crates/r2d2) connection pool.
///
/// This helper can support all adapter that implement `r2d2`'s [ManageConnection](https://docs.rs/r2d2/0.8.2/r2d2/trait.ManageConnection.html) trait.
///
/// You can see the list supported of adapter here: [https://github.com/sfackler/r2d2](https://github.com/sfackler/r2d2)
pub struct Conn<T>
where
    T: ManageConnection + 'static,
{
    pooled_connection: PooledConnection<T>,
}

impl<T> Conn<T>
where
    T: ManageConnection + 'static,
{
    /// This method initialize connection pool, which can be managed by rocket.
    ///
    /// These are examples of initializing connection pool using different r2d2 adapter
    /// ### Sqlite
    /// ```rust,ignore
    /// ...
    /// extern crate r2d2;
    /// extern crate r2d2_sqlite;
    /// ...
    /// use std::env::current_dir;
    /// use rocket_contrib::conn::Conn;
    /// use r2d2_sqlite::SqliteConnectionManager;
    ///
    /// fn main() {
    ///     let current_dir = current_dir().unwrap();
    ///     let sqlite_db_path = current_dir.join("path/to/sqlite_file.sqlite");
    ///     let sqlite_manager = SqliteConnectionManager::file(sqlite_db_path);
    ///     let sqlite_pool = Conn::init_pool(sqlite_manager, 4).unwrap();
    ///     ...
    ///     rocket::ignite()
    ///         .manage(sqlite_pool)
    ///     ...
    /// }
    /// ```
    /// ### Diesel
    /// ```rust,ignore
    /// ...
    /// extern crate r2d2;
    /// extern crate r2d2_diesel;
    /// ...
    /// use rocket_contrib::conn::Conn;
    /// use r2d2_diesel::ConnectionManager;
    ///
    /// fn main() {
    ///     let postgres_url = "postgres://user:password@hostname/rocket_development".to_string();
    ///     let pg_manager = ConnectionManager::<PgConnection>::new(postgres_url);
    ///     let diesel_pool = Conn::init_pool(pg_manager, 8).unwrap();
    ///     ...
    ///     rocket::ignite()
    ///         .manage(diesel_pool)
    ///     ...
    /// }
    /// ```
    /// ### Redis
    /// ```rust,ignore
    /// ...
    /// extern crate r2d2;
    /// extern crate r2d2_redis;
    /// extern crate redis;
    /// ...
    /// use r2d2_redis::RedisConnectionManager;
    /// ...
    /// fn main() {
    /// ...
    ///     let redis_manager = RedisConnectionManager::new("redis://localhost").unwrap();
    ///     let redis_pool = Conn::init_pool(redis_manager, 4).unwrap();
    ///     ...
    ///     rocket::ignite()
    ///         .manage(redis_pool)
    ///     ...
    /// }
    pub fn init_pool(manager: T, max_size: u32) -> Result<Pool<T>, r2d2::Error> {
        let pool = Pool::builder().max_size(max_size).build(manager)?;
        Ok(pool)
    }
}

impl<T> Deref for Conn<T>
where
    T: ManageConnection + 'static,
{
    type Target = PooledConnection<T>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.pooled_connection
    }
}

impl<'a, 'r, T> FromRequest<'a, 'r> for Conn<T>
where
    T: ManageConnection + 'static,
{
    type Error = ();
    /// This is `FromRequest` implementation for `Conn`. It is used retrieve connection from connection pool managed by Rocket.
    ///
    /// You might have to dereference it to be able to use the connection, depending on the adapter implementation.
    ///
    /// These are examples of using this guard to make database connection:
    ///
    /// ### Sqlite
    /// ```rust,ignore
    ///
    /// use r2d2_sqlite::SqliteConnectionManager;
    /// use rocket_contrib::conn::Conn;
    /// pub struct Person {
    ///     id: i32,
    ///     name: String,
    /// }
    /// #[get("/sqlite_example")]
    /// pub fn sqlite_example(conn: Conn<SqliteConnectionManager>) -> String {
    ///     let mut stmt = conn.prepare("SELECT id, name FROM person LIMIT 1").unwrap();
    ///     let person_iter = stmt.query_map(&[], |row| Person {
    ///         id: row.get(0),
    ///         name: row.get(1),
    ///     }).unwrap();
    ///     let person = &person_iter.last().unwrap().unwrap();
    ///     format!("Hello user: {} with id: {}", person.name, person.id)
    /// }
    /// ```
    /// ### Diesel
    /// ```rust,ignore
    /// use std::ops::Deref;
    /// use diesel::prelude::*;
    /// use diesel::PgConnection;
    /// use r2d2_diesel::ConnectionManager;
    /// use rocket_contrib::conn::Conn;
    /// use schema::users::dsl::users;
    /// use schema::users::columns::{id, username};
    ///
    /// #[derive(Queryable)]
    /// pub struct User {
    ///     pub id: i32,
    ///     pub username: String,
    /// }
    ///
    /// #[get("/diesel_example")]
    /// pub fn diesel_example(conn: Conn<ConnectionManager<PgConnection>>) -> String {
    ///     let selected_user = users
    ///         .select((id, username))
    ///         .order(id.asc())
    ///         .first::<User>(&**conn.deref())
    ///         .optional()
    ///         .expect("Failed to load user");
    ///     let user = selected_user.unwrap();
    ///     format!("Hello user: {} with id: {}", user.username, user.id)
    /// }
    /// ```
    /// ### Redis example
    /// ```rust,ignore
    /// use std::ops::Deref;
    /// use r2d2_redis::RedisConnectionManager;
    /// use rocket_contrib::conn::Conn;
    /// use redis;
    ///
    /// #[get("/redis_example")]
    /// pub fn redis_example(conn: Conn<RedisConnectionManager>) -> String {
    ///     let reply = redis::cmd("PING").query::<String>(&**conn.deref()).unwrap();
    ///     format!("Redis query result: {}", reply)
    /// }
    /// ```
    ///

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Conn<T>, ()> {
        let pool = request.guard::<State<Pool<T>>>()?;

        match pool.get() {
            Ok(conn) => Outcome::Success(Conn {
                pooled_connection: conn,
            }),
            Err(_) => Outcome::Failure((Status::ServiceUnavailable, ())),
        }
    }
}
