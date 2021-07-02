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
  2. Add a `&State<T>` type to any request handler, where `T` is the type of the
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

rocket::build().manage(HitCount { count: AtomicUsize::new(0) });
```

The `manage` method can be called any number of times as long as each call
refers to a value of a different type. For instance, to have Rocket manage both
a `HitCount` value and a `Config` value, we can write:

```rust
# use std::sync::atomic::AtomicUsize;
# struct HitCount { count: AtomicUsize }
# type Config = &'static str;
# let user_input = "input";

rocket::build()
    .manage(HitCount { count: AtomicUsize::new(0) })
    .manage(Config::from(user_input));
```

### Retrieving State

State that is being managed by Rocket can be retrieved via the
[`&State`](@api/rocket/struct.State.html) type: a [request
guard](../requests/#request-guards) for managed state. To use the request guard,
add a `&State<T>` type to any request handler, where `T` is the type of the
managed state. For example, we can retrieve and respond with the current
`HitCount` in a `count` route as follows:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

# use std::sync::atomic::{AtomicUsize, Ordering};
# struct HitCount { count: AtomicUsize }

use rocket::State;

#[get("/count")]
fn count(hit_count: &State<HitCount>) -> String {
    let current_count = hit_count.count.load(Ordering::Relaxed);
    format!("Number of visits: {}", current_count)
}
```

You can retrieve more than one `&State` type in a single route as well:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

# struct HitCount;
# struct Config;
# use rocket::State;

#[get("/state")]
fn state(hit_count: &State<HitCount>, config: &State<Config>) { /* .. */ }
```

! warning

  If you request a `&State<T>` for a `T` that is not `managed`, Rocket will
  refuse to start your application. This prevents what would have been an
  unmanaged state runtime error. Unmanaged state is detected at runtime through
  [_sentinels_](@api/rocket/trait.Sentinel.html), so there are limitations. If a
  limitation is hit, Rocket still won't call an the offending route. Instead,
  Rocket will log an error message and return a **500** error to the client.

You can find a complete example using the `HitCount` structure in the [state
example on GitHub](@example/state) and learn more about the [`manage`
method](@api/rocket/struct.Rocket.html#method.manage) and [`State`
type](@api/rocket/struct.State.html) in the API docs.

### Within Guards

Because `State` is itself a request guard, managed state can be retrieved from
another request guard's implementation using either [`Request::guard()`] or
[`Rocket::state()`]. In the following code example, the `Item` request guard
retrieves `MyConfig` from managed state using both methods:

```rust
use rocket::State;
use rocket::request::{self, Request, FromRequest};
use rocket::outcome::IntoOutcome;

# struct MyConfig { user_val: String };
struct Item<'r>(&'r str);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Item<'r> {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, ()> {
        // Using `State` as a request guard. Use `inner()` to get an `'r`.
        let outcome = request.guard::<&State<MyConfig>>().await
            .map(|my_config| Item(&my_config.user_val));

        // Or alternatively, using `Rocket::state()`:
        let outcome = request.rocket().state::<MyConfig>()
            .map(|my_config| Item(&my_config.user_val))
            .or_forward(());

        outcome
    }
}
```


[`Request::guard()`]: @api/rocket/struct.Request.html#method.guard
[`Rocket::state()`]: @api/rocket/struct.Rocket.html#method.state

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
# #[macro_use] extern crate rocket;
# fn main() {}
# use std::sync::atomic::{AtomicUsize, Ordering};

use rocket::request::{self, Request, FromRequest};

/// A global atomic counter for generating IDs.
static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A type that represents a request's ID.
struct RequestId(pub usize);

/// Returns the current request's ID, assigning one only as necessary.
#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r RequestId {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        // The closure passed to `local_cache` will be executed at most once per
        // request: the first time the `RequestId` guard is used. If it is
        // requested again, `local_cache` will return the same value.
        request::Outcome::Success(request.local_cache(|| {
            RequestId(ID_COUNTER.fetch_add(1, Ordering::Relaxed))
        }))
    }
}

#[get("/")]
fn id(id: &RequestId) -> String {
    format!("This is request #{}.", id.0)
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
  3. Use the request guard to retrieve and use a connection in a handler.

Presently, Rocket provides built-in support for the following databases:

<!-- Note: Keep this table in sync with contrib/sync_db_pools/src/lib.rs -->
| Kind     | Driver                | Version   | `Poolable` Type                | Feature                |
|----------|-----------------------|-----------|--------------------------------|------------------------|
| MySQL    | [Diesel]              | `1`       | [`diesel::MysqlConnection`]    | `diesel_mysql_pool`    |
| Postgres | [Diesel]              | `1`       | [`diesel::PgConnection`]       | `diesel_postgres_pool` |
| Postgres | [Rust-Postgres]       | `0.19`    | [`postgres::Client`]           | `postgres_pool`        |
| Sqlite   | [Diesel]              | `1`       | [`diesel::SqliteConnection`]   | `diesel_sqlite_pool`   |
| Sqlite   | [`Rusqlite`]          | `0.24`    | [`rusqlite::Connection`]       | `sqlite_pool`          |
| Memcache | [`memcache`]          | `0.15`    | [`memcache::Client`]           | `memcache_pool`        |

[`r2d2`]: https://crates.io/crates/r2d2
[Diesel]: https://diesel.rs
[`rusqlite::Connection`]: https://docs.rs/rusqlite/0.23.0/rusqlite/struct.Connection.html
[`diesel::SqliteConnection`]: https://docs.diesel.rs/diesel/prelude/struct.SqliteConnection.html
[`postgres::Client`]: https://docs.rs/postgres/0.19/postgres/struct.Client.html
[`diesel::PgConnection`]: https://docs.diesel.rs/diesel/pg/struct.PgConnection.html
[`diesel::MysqlConnection`]: https://docs.diesel.rs/diesel/mysql/struct.MysqlConnection.html
[`Rusqlite`]: https://github.com/jgallagher/rusqlite
[Rust-Postgres]: https://github.com/sfackler/rust-postgres
[`diesel::PgConnection`]: https://docs.diesel.rs/diesel/pg/struct.PgConnection.html
[`memcache`]: https://github.com/aisk/rust-memcache
[`memcache::Client`]: https://docs.rs/memcache/0.15/memcache/struct.Client.html

### Usage

To connect your Rocket application to a given database, first identify the
"Kind" and "Driver" in the table that matches your environment. The feature
corresponding to your database type must be enabled. This is the feature
identified in the "Feature" column. For instance, for Diesel-based SQLite
databases, you'd write in `Cargo.toml`:

```toml
[dependencies.rocket_sync_db_pools]
version = "0.1.0-rc.1"
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
database that you configured in the previous step as the only parameter. You
will need to either add `#[macro_use] extern crate rocket_sync_db_pools` to the
crate root or have a `use rocket_sync_db_pools::database` in scope, otherwise
the `database` attribute will not be available. Finally, attach the fairing
returned by `YourType::fairing()`, which was generated by the `#[database]`
attribute:

```rust
# #[macro_use] extern crate rocket;

use rocket_sync_db_pools::{diesel, database};

#[database("sqlite_logs")]
struct LogsDbConn(diesel::SqliteConnection);

#[launch]
fn rocket() -> _ {
    rocket::build().attach(LogsDbConn::fairing())
}
```

That's it! Whenever a connection to the database is needed, use your type as a
request guard. The database can be accessed by calling the `run` method:

```rust
# #[macro_use] extern crate rocket;
# fn main() {}

# use rocket_sync_db_pools::{diesel, database};

# #[database("sqlite_logs")]
# struct LogsDbConn(diesel::SqliteConnection);
# type Logs = ();

#[get("/logs/<id>")]
async fn get_logs(conn: LogsDbConn, id: usize) -> Logs {
    # /*
    conn.run(|c| logs::filter(id.eq(log_id)).load(c)).await
    # */
}
```

! note The above examples uses [Diesel] with some fictional `Logs` type.

  The example above contains the use of a `Logs` type that is application
  specific and not built into Rocket. It also uses [Diesel]'s query-building
  syntax. Rocket does not provide an ORM. It is up to you to decide how to model
  your application's data.

<!---->

! note: Rocket wraps synchronous databases in an `async` API.

  The database engines supported by `#[database]` are *synchronous*. Normally,
  using such a database would block the thread of execution. To prevent this,
  the `run()` function automatically uses a thread pool so that database access
  does not interfere with other in-flight requests. See
  [Multitasking](../overview/#multitasking) for more information on why this is
  necessary.

If your application uses features of a database engine that are not available
by default, for example support for `chrono` or `uuid`, you may enable those
features by adding them in `Cargo.toml` like so:

```toml
[dependencies]
postgres = { version = "0.15", features = ["with-chrono"] }
```

For more on Rocket's sanctioned database support, see the
[`rocket_sync_db_pools`] library documentation. For examples of CRUD-like "blog"
JSON APIs backed by a SQLite database driven by each of `sqlx`, `diesel`, and
`rusqlite` with migrations run automatically for the former two drivers and
Rocket's database support use for the latter two drivers, see the [databases
example](@example/databases).

[`rocket_sync_db_pools`]: @api/rocket_sync_db_pools/index.html
