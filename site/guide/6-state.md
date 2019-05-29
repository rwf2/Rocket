# State

Many web applications have a need to maintain state. This can be as simple as
maintaining a counter for the number of visits or as complex as needing to
access job queues and multiple databases. Rocket provides the tools to enable
these kinds of interactions in a safe and simple manner.

## Managed State

The enabling feature for maintaining state is _managed state_. Managed state, as
the name implies, is state that Rocket manages for your application. The state
is managed on a per-type basis: Rocket will manage at most one value of a given
type.

The process for using managed state is simple:

  1. Call `manage` on the `Rocket` instance corresponding to your application
     with the initial value of the state.
  2. Add a `State<T>` type to any request handler, where `T` is the type of the
     value passed into `manage`.

! note: All managed state must be thread-safe.

  Because Rocket automatically multithreads your application, handlers can
  concurrently access managed state. As a result, managed state must be
  thread-safe. Thanks to Rust, this condition is checked at compile-time by
  ensuring that the type of values you store in managed state implement `Send` +
  `Sync`.

### Adding State

To instruct Rocket to manage state for your application, call the
[`manage`](@api/rocket/struct.Rocket.html#method.manage) method
on an instance of `Rocket`. For example, to ask Rocket to manage a `HitCount`
structure with an internal `AtomicUsize` with an initial value of `0`, we can
write the following:

```rust
use std::sync::atomic::AtomicUsize;

struct HitCount {
    count: AtomicUsize
}

rocket::ignite().manage(HitCount { count: AtomicUsize::new(0) });
```

The `manage` method can be called any number of times as long as each call
refers to a value of a different type. For instance, to have Rocket manage both
a `HitCount` value and a `Config` value, we can write:

```rust
rocket::ignite()
    .manage(HitCount { count: AtomicUsize::new(0) })
    .manage(Config::from(user_input));
```

### Retrieving State

State that is being managed by Rocket can be retrieved via the
[`State`](@api/rocket/struct.State.html) type: a [request
guard](../requests/#request-guards) for managed state. To use the request
guard, add a `State<T>` type to any request handler, where `T` is the type of
the managed state. For example, we can retrieve and respond with the current
`HitCount` in a `count` route as follows:

```rust
use rocket::State;

#[get("/count")]
fn count(hit_count: State<HitCount>) -> String {
    let current_count = hit_count.count.load(Ordering::Relaxed);
    format!("Number of visits: {}", current_count)
}
```

You can retrieve more than one `State` type in a single route as well:

```rust
#[get("/state")]
fn state(hit_count: State<HitCount>, config: State<Config>) -> T { ... }
```

! warning

  If you request a `State<T>` for a `T` that is not `managed`, Rocket won't call
  the offending route. Instead, Rocket will log an error message and return a
  **500** error to the client.

You can find a complete example using the `HitCount` structure in the [state
example on GitHub](@example/state) and learn more about the [`manage`
method](@api/rocket/struct.Rocket.html#method.manage) and [`State`
type](@api/rocket/struct.State.html) in the API docs.

### Within Guards

It can also be useful to retrieve managed state from a `FromRequest`
implementation. To do so, simply invoke `State<T>` as a guard using the
[`Request::guard()`] method.

```rust
fn from_request(req: &'a Request<'r>) -> request::Outcome<T, ()> {
    let hit_count_state = req.guard::<State<HitCount>>()?;
    let current_count = hit_count_state.count.load(Ordering::Relaxed);
    ...
}
```

[`Request::guard()`]: @api/rocket/struct.Request.html#method.guard

## Request-Local State

While managed state is *global* and available application-wide, request-local
state is *local* to a given request, carried along with the request, and dropped
once the request is completed. Request-local state can be used whenever a
`Request` is available, such as in a fairing, a request guard, or a responder.

Request-local state is *cached*: if data of a given type has already been
stored, it will be reused. This is especially useful for request guards that
might be invoked multiple times during routing and processing of a single
request, such as those that deal with authentication.

As an example, consider the following request guard implementation for
`RequestId` that uses request-local state to generate and expose a unique
integer ID per request:

```rust
/// A global atomic counter for generating IDs.
static request_id_counter: AtomicUsize = AtomicUsize::new(0);

/// A type that represents a request's ID.
struct RequestId(pub usize);

/// Returns the current request's ID, assigning one only as necessary.
impl<'a, 'r> FromRequest<'a, 'r> for RequestId {
    fn from_request(request: &'a Request<'r>) -> request::Outcome {
        // The closure passed to `local_cache` will be executed at most once per
        // request: the first time the `RequestId` guard is used. If it is
        // requested again, `local_cache` will return the same value.
        Outcome::Success(request.local_cache(|| {
            RequestId(request_id_counter.fetch_add(1, Ordering::Relaxed))
        }))
    }
}
```

Note that, without request-local state, it would not be possible to:

  1. Associate a piece of data, here an ID, directly with a request.
  2. Ensure that a value is generated at most once per request.

For more examples, see the [`FromRequest` request-local state] documentation,
which uses request-local state to cache expensive authentication and
authorization computations, and the [`Fairing`] documentation, which uses
request-local state to implement request timing.

[`FromRequest` request-local state]: @api/rocket/request/trait.FromRequest.html#request-local-state
[`Fairing`]: @api/rocket/fairing/trait.Fairing.html#request-local-state

## Databases

Rocket includes built-in, ORM-agnostic support for databases. In particular,
Rocket provides a procedural macro that allows you to easily connect your Rocket
application to databases through connection pools. A _database connection pool_
is a data structure that maintains active database connections for later use in
the application. This implementation of connection pooling support is based on
[`r2d2`] and exposes connections through request guards. Databases are
individually configured through Rocket's regular configuration mechanisms: a
`Rocket.toml` file, environment variables, or procedurally.

Connecting your Rocket application to a database using this library occurs in
three simple steps:

  1. Configure the databases in `Rocket.toml`.
  2. Associate a request guard type and fairing with each database.
  3. Use the request guard to retrieve a connection in a handler.

Presently, Rocket provides built-in support for the following databases:

<!-- Note: Keep this table in sync with contrib/lib/src/databases.rs -->
| Kind     | Driver                | Version   | `Poolable` Type                | Feature                |
|----------|-----------------------|-----------|--------------------------------|------------------------|
| MySQL    | [Diesel]              | `1`       | [`diesel::MysqlConnection`]    | `diesel_mysql_pool`    |
| MySQL    | [`rust-mysql-simple`] | `16`      | [`mysql::conn`]                | `mysql_pool`           |
| Postgres | [Diesel]              | `1`       | [`diesel::PgConnection`]       | `diesel_postgres_pool` |
| Postgres | [Rust-Postgres]       | `0.15`    | [`postgres::Connection`]       | `postgres_pool`        |
| Sqlite   | [Diesel]              | `1`       | [`diesel::SqliteConnection`]   | `diesel_sqlite_pool`   |
| Sqlite   | [`Rustqlite`]         | `0.16`    | [`rusqlite::Connection`]       | `sqlite_pool`          |
| Neo4j    | [`rusted_cypher`]     | `1`       | [`rusted_cypher::GraphClient`] | `cypher_pool`          |
| Redis    | [`redis-rs`]          | `0.10`    | [`redis::Connection`]          | `redis_pool`           |
| MongoDB  | [`mongodb`]           | `0.3.12`  | [`mongodb::db::Database`]      | `mongodb_pool`         |
| Memcache | [`memcache`]          | `0.11`    | [`memcache::Client`]           | `memcache_pool`        |

[`r2d2`]: https://crates.io/crates/r2d2
[Diesel]: https://diesel.rs
[`redis::Connection`]: https://docs.rs/redis/0.10.0/redis/struct.Connection.html
[`rusted_cypher::GraphClient`]: https://docs.rs/rusted_cypher/1.1.0/rusted_cypher/graph/struct.GraphClient.html
[`rusqlite::Connection`]: https://docs.rs/rusqlite/0.16.0/rusqlite/struct.Connection.html
[`diesel::SqliteConnection`]: http://docs.diesel.rs/diesel/prelude/struct.SqliteConnection.html
[`postgres::Connection`]: https://docs.rs/postgres/0.15.2/postgres/struct.Connection.html
[`diesel::PgConnection`]: http://docs.diesel.rs/diesel/pg/struct.PgConnection.html
[`mysql::conn`]: https://docs.rs/mysql/16.0.0/mysql/struct.Conn.html
[`diesel::MysqlConnection`]: http://docs.diesel.rs/diesel/mysql/struct.MysqlConnection.html
[`redis-rs`]: https://github.com/mitsuhiko/redis-rs
[`rusted_cypher`]: https://github.com/livioribeiro/rusted-cypher
[`Rustqlite`]: https://github.com/jgallagher/rusqlite
[Rust-Postgres]: https://github.com/sfackler/rust-postgres
[`rust-mysql-simple`]: https://github.com/blackbeam/rust-mysql-simple
[`diesel::PgConnection`]: http://docs.diesel.rs/diesel/pg/struct.PgConnection.html
[`mongodb`]: https://github.com/mongodb-labs/mongo-rust-driver-prototype
[`mongodb::db::Database`]: https://docs.rs/mongodb/0.3.12/mongodb/db/type.Database.html
[`memcache`]: https://github.com/aisk/rust-memcache
[`memcache::Client`]: https://docs.rs/memcache/0.11.0/memcache/struct.Client.html

### Usage

To connect your Rocket application to a given database, first identify the
"Kind" and "Driver" in the table that matches your environment. The feature
corresponding to your database type must be enabled. This is the feature
identified in the "Feature" column. For instance, for Diesel-based SQLite
databases, you'd write in `Cargo.toml`:

```toml
[dependencies.rocket_contrib]
version = "0.5.0-dev"
default-features = false
features = ["diesel_sqlite_pool"]
```

Then, in `Rocket.toml` or the equivalent via environment variables, configure
the URL for the database in the `databases` table:

```toml
[global.databases]
sqlite_logs = { url = "/path/to/database.sqlite" }
```

In your application's source code, create a unit-like struct with one internal
type. This type should be the type listed in the "`Poolable` Type" column. Then
decorate the type with the `#[database]` attribute, providing the name of the
database that you configured in the previous step as the only parameter.
Finally, attach the fairing returned by `YourType::fairing()`, which was
generated by the `#[database]` attribute:

```rust
#[macro_use] extern crate rocket_contrib;

use rocket_contrib::databases::diesel;

#[database("sqlite_logs")]
struct LogsDbConn(diesel::SqliteConnection);

fn main() {
    rocket::ignite()
       .attach(LogsDbConn::fairing())
       .launch();
}
```

That's it! Whenever a connection to the database is needed, use your type as a
request guard:

```rust
#[get("/logs/<id>")]
fn get_logs(conn: LogsDbConn, id: usize) -> Result<Logs> {
    logs::filter(id.eq(log_id)).load(&*conn)
}
```

If your application uses features of a database engine that are not available
by default, for example support for `chrono` or `uuid`, you may enable those
features by adding them in `Cargo.toml` like so:

```toml
[dependencies]
postgres = { version = "0.15", features = ["with-chrono"] }
```

For more on Rocket's built-in database support, see the
[`rocket_contrib::databases`] module documentation.

[`rocket_contrib::databases`]: @api/rocket_contrib/databases/index.html
