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

### Adding State

To instruct Rocket to manage state for your application, call the
[`manage`](https://api.rocket.rs/rocket/struct.Rocket.html#method.manage) method
on an instance of `Rocket`. For example, to ask Rocket to manage a `HitCount`
structure with an internal `AtomicUsize` with an initial value of `0`, we can
write the following:

```rust
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
[`State`](https://api.rocket.rs/rocket/struct.State.html) type: a [request
guard](/guide/requests/#request-guards) for managed state. To use the request
guard, add a `State<T>` type to any request handler, where `T` is the type of
the managed state. For example, we can retrieve and respond with the current
`HitCount` in a `count` route as follows:

```rust
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

[`Request::guard()`]: https://api.rocket.rs/rocket/struct.Request.html#method.guard

### Unmanaged State

If you request a `State<T>` for a `T` that is not `managed`, Rocket won't call
the offending route. Instead, Rocket will log an error message and return a
**500** error to the client.

While this behavior is 100% safe, it isn't fun to return **500** errors to
clients, especially when the issue can be easily avoided. Because of this,
Rocket tries to prevent an application with unmanaged state from ever running
via the `unmanaged_state` lint. The lint reads through your code at compile-time
and emits a warning when a `State<T>` request guard is being used in a mounted
route for a type `T` that isn't being managed.

As an example, consider the following short application using our `HitCount`
type from previous examples:

```rust
#[get("/count")]
fn count(hit_count: State<HitCount>) -> String {
    let current_count = hit_count.count.load(Ordering::Relaxed);
    format!("Number of visits: {}", current_count)
}

fn main() {
    rocket::ignite()
        .manage(Config::from(user_input))
        .launch()
}
```

The application is buggy: a value for `HitCount` isn't being `managed`, but a
`State<HitCount>` type is being requested in the `count` route. When we compile
this application, Rocket emits the following warning:

```rust
warning: HitCount is not currently being managed by Rocket
 --> src/main.rs:2:17
  |
2 | fn count(hit_count: State<HitCount>) -> String {
  |                           ^^^^^^^^
  |
  = note: this State request guard will always fail
help: maybe add a call to 'manage' here?
 --> src/main.rs:8:5
  |
8 |     rocket::ignite()
  |     ^^^^^^^^^^^^^^^^
```

The `unmanaged_state` lint isn't perfect. In particular, it cannot track calls
to `manage` across function boundaries. Because of this, you may find yourself
with incorrect warnings. You can disable the lint on a per-route basis by adding
`#[allow(unmanaged_state)]` to a route handler. If you wish to disable the lint
globally, add `#![allow(unmanaged_state)]` to your crate attributes.

You can find a complete example using the `HitCount` structure in the [state
example on
GitHub](https://github.com/SergioBenitez/Rocket/tree/v0.4.0-dev/examples/state) and
learn more about the [`manage`
method](https://api.rocket.rs/rocket/struct.Rocket.html#method.manage) and
[`State` type](https://api.rocket.rs/rocket/struct.State.html) in the API docs.

## Databases

While Rocket doesn't have built-in support for databases yet, you can combine a
few external libraries to get native-feeling access to databases in a Rocket
application. Let's take a look at how we might integrate Rocket with two common
database libraries: [`diesel`], a type-safe ORM and query builder, and [`r2d2`],
a library for connection pooling.

Our approach will be to have Rocket manage a pool of database connections using
managed state and then implement a request guard that retrieves one connection.
This will allow us to get access to the database in a handler by simply adding a
`DbConn` argument:

```rust
#[get("/users")]
fn handler(conn: DbConn) { ... }
```

[`diesel`]: http://diesel.rs/
[`r2d2`]: https://docs.rs/r2d2/0.7.2/r2d2/

### Dependencies

To get started, we need to depend on the `diesel` and `r2d2` crates. For
detailed information on how to use Diesel, please see the [Diesel getting
started guide](http://diesel.rs/guides/getting-started/). For this example, we
use the following dependencies:

```
[dependencies]
rocket = "0.4.0-dev"
diesel = { version = "*", features = ["sqlite"] }
diesel_codegen = { version = "*", features = ["sqlite"] }
r2d2-diesel = "*"
r2d2 = "*"
```

Your `diesel` dependency information will differ. In particular, you should
specify the latest versions of these libraries as opposed to using a `*`. The
crates are imported as well:

```rust
extern crate rocket;
#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_codegen;
extern crate r2d2_diesel;
extern crate r2d2;
```

### Managed Pool

The first step is to initialize a pool of database connections. The `init_pool`
function below uses `r2d2` to create a new pool of database connections. Diesel
advocates for using a `DATABASE_URL` environment variable to set the database
URL, and we use the same convention here. Excepting the long-winded types, the
code is fairly straightforward: the `DATABASE_URL` environment variable is
stored in the `DATABASE_URL` static, and an `r2d2::Pool` is created using the
default configuration parameters and a Diesel `SqliteConnection`
`ConnectionManager`.

```rust
use diesel::sqlite::SqliteConnection;
use r2d2_diesel::ConnectionManager;

// An alias to the type for a pool of Diesel SQLite connections.
type Pool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

// The URL to the database, set via the `DATABASE_URL` environment variable.
static DATABASE_URL: &'static str = env!("DATABASE_URL");

/// Initializes a database pool.
fn init_pool() -> Pool {
    let config = r2d2::Config::default();
    let manager = ConnectionManager::<SqliteConnection>::new(DATABASE_URL);
    r2d2::Pool::new(config, manager).expect("db pool")
}
```

We then use managed state to have Rocket manage the pool for us:

```rust
fn main() {
    rocket::ignite()
        .manage(init_pool())
        .launch();
}
```

### Connection Guard

The second and final step is to implement a request guard that retrieves a
single connection from the managed connection pool. We create a new type,
`DbConn`, that wraps an `r2d2` pooled connection. We then implement
`FromRequest` for `DbConn` so that we can use it as a request guard. Finally, we
implement `Deref` with a target of `SqliteConnection` so that we can
transparently use an `&*DbConn` as an `&SqliteConnection`.

```rust
use std::ops::Deref;
use rocket::http::Status;
use rocket::request::{self, FromRequest};
use rocket::{Request, State, Outcome};

// Connection request guard type: a wrapper around an r2d2 pooled connection.
pub struct DbConn(pub r2d2::PooledConnection<ConnectionManager<SqliteConnection>>);

/// Attempts to retrieve a single connection from the managed database pool. If
/// no pool is currently managed, fails with an `InternalServerError` status. If
/// no connections are available, fails with a `ServiceUnavailable` status.
impl<'a, 'r> FromRequest<'a, 'r> for DbConn {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<DbConn, ()> {
        let pool = request.guard::<State<Pool>>()?;
        match pool.get() {
            Ok(conn) => Outcome::Success(DbConn(conn)),
            Err(_) => Outcome::Failure((Status::ServiceUnavailable, ()))
        }
    }
}

// For the convenience of using an &DbConn as an &SqliteConnection.
impl Deref for DbConn {
    type Target = SqliteConnection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
```

### Usage

With these two pieces in place, we can use `DbConn` as a request guard in any
handler or other request guard implementation, giving our application access to
a database. As a simple example, we might write a route that returns a JSON
array of some `Task` structures that are fetched from a database:

```rust
#[get("/tasks")]
fn get_tasks(conn: DbConn) -> QueryResult<Json<Vec<Task>>> {
    all_tasks.order(tasks::id.desc())
        .load::<Task>(&*conn)
        .map(|tasks| Json(tasks))
}
```
