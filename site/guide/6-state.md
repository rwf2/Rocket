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

Rocket includes built-in, ORM-agnostic support for databases via
[`rocket_db_pools`]. The library simplifies accessing one or more databases via
connection pools: data structures that maintain active database connections for
use in the application. Database configuration occurs via Rocket's regular
[configuration](../configuration) mechanisms.

Connecting your Rocket application to a database using `rocket_db_pools` happens
in three simple steps:

1. Choose your database(s) from the [supported database driver list]. Add
   `rocket_db_pools` as a dependency in `Cargo.toml` with respective database
   driver feature(s) enabled:

   ```toml
   [dependencies.rocket_db_pools]
   version = "0.1.0-rc.2"
   features = ["sqlx_sqlite"]
   ```

2. Choose a name for your database, here `sqlite_logs`. [Configure]  _at least_
   a URL for the database under `databases.$name` (here, in `Rocket.toml`),
   where `$name` is your choice of database name:

   ```toml
   [default.databases.sqlite_logs]
   url = "/path/to/database.sqlite"
   ```

3. [Derive `Database`] for a unit `Type` (`Logs` here) which wraps the selected
   driver's `Pool` type from the [supported database driver list]. Decorated the
   struct with `#[database("$name")]` with the `$name` from `2.`. Attach
   `$Type::init()` to your application's `Rocket` to initialize the database
   pool and use [`Connection<$Type>`] as a request guard to retrieve an active
   database connection:

   ```rust
   #[macro_use] extern crate rocket;

   use rocket_db_pools::{Database, Connection};
   use rocket_db_pools::sqlx::{self, Row};

   #[derive(Database)]
   #[database("sqlite_logs")]
   struct Logs(sqlx::SqlitePool);

   #[get("/<id>")]
   async fn read(mut db: Connection<Logs>, id: i64) -> Option<String> {
       sqlx::query("SELECT content FROM logs WHERE id = ?").bind(id)
           .fetch_one(&mut *db).await
           .and_then(|r| Ok(r.try_get(0)?))
           .ok()
   }

   #[launch]
   fn rocket() -> _ {
       rocket::build().attach(Logs::init()).mount("/", routes![read])
   }
   ```

For complete usage details, see [`rocket_db_pools`].

[`rocket_db_pools`]: @api/rocket_db_pools/index.html
[supported database driver list]: @api/rocket_db_pools/index.html#supported-drivers
[database driver features]: @api/rocket_db_pools/index.html#supported-drivers
[`Pool`]: @api/rocket_db_pools/index.html#supported-drivers
[Configure]: @api/rocket_db_pools/index.html#configuration
[Derive `Database`]: @api/rocket_db_pools/derive.Database.html
[`Connection<$Type>`]: @api/rocket_db_pools/struct.Connection.html

### Driver Features

Only the minimal features for each driver crate are enabled by
`rocket_db_pools`. To use additional driver functionality exposed via its
crate's features, you'll need to depend on the crate directly with those
features enabled in `Cargo.toml`:

```toml
[dependencies.sqlx]
version = "0.6"
default-features = false
features = ["macros", "offline", "migrate"]

[dependencies.rocket_db_pools]
version = "0.1.0-rc.2"
features = ["sqlx_sqlite"]
```

### Synchronous ORMs

While [`rocket_db_pools`] provides support for `async` ORMs and should thus be
the preferred solution, Rocket also provides support for synchronous, blocking
ORMs like [Diesel] via the [`rocket_sync_db_pools`] library, which you may wish
to explore. Usage is similar, but not identical, to `rocket_db_pools`. See the
crate docs for complete usage details.

[`rocket_sync_db_pools`]: @api/rocket_sync_db_pools/index.html
[diesel]: https://diesel.rs/

### Examples

For examples of CRUD-like "blog" JSON APIs backed by a SQLite database driven by
each of `sqlx`, `diesel`, and `rusqlite`, with migrations run automatically for
the former two drivers, see the [databases example](@example/databases). The
`sqlx` example uses `rocket_db_pools` while the `diesel` and `rusqlite` examples
use `rocket_sync_db_pools`.
