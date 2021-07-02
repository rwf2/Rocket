# Overview

Rocket provides primitives to build web servers and applications with Rust:
Rocket provides routing, pre-processing of requests, and post-processing of
responses; the rest is up to you. Your application code instructs Rocket on what
to pre-process and post-process and fills the gaps between pre-processing and
post-processing.

## Lifecycle

Rocket's main task is to listen for incoming web requests, dispatch the request
to the application code, and return a response to the client. We call the
process that goes from request to response the "lifecycle". We summarize the
lifecycle as the following sequence of steps:

  1. **Routing**

     Rocket parses an incoming HTTP request into native structures that your
     code operates on indirectly. Rocket determines which request handler to
     invoke by matching against route attributes declared in your application.

  2. **Validation**

     Rocket validates the incoming request against types and guards present in
     the matched route. If validation fails, Rocket _forwards_ the request to
     the next matching route or calls an _error handler_.

  3. **Processing**

     The request handler associated with the route is invoked with validated
     arguments. This is the main business logic of an application. Processing
     completes by returning a `Response`.

  4. **Response**

     The returned `Response` is processed. Rocket generates the appropriate HTTP
     response and sends it to the client. This completes the lifecycle. Rocket
     continues listening for requests, restarting the lifecycle for each
     incoming request.

The remainder of this section details the _routing_ phase as well as additional
components needed for Rocket to begin dispatching requests to request handlers.
The sections following describe the request and response phases as well as other
components of Rocket.

## Routing

Rocket applications are centered around routes and handlers. A _route_ is a
combination of:

  * A set of parameters to match an incoming request against.
  * A handler to process the request and return a response.

A _handler_ is simply a function that takes an arbitrary number of arguments and
returns any arbitrary type.

The parameters to match against include static paths, dynamic paths, path
segments, forms, query strings, request format specifiers, and body data. Rocket
uses attributes, which look like function decorators in other languages, to make
declaring routes easy. Routes are declared by annotating a function, the
handler, with the set of parameters to match against. A complete route
declaration looks like this:

```rust
# #[macro_use] extern crate rocket;

#[get("/world")]              // <- route attribute
fn world() -> &'static str {  // <- request handler
    "hello, world!"
}
```

This declares the `world` route to match against the static path `"/world"` on
incoming `GET` requests. Instead of `#[get]`, we could have used `#[post]` or
`#[put]` for other HTTP methods, or `#[catch]` for serving [custom error
pages](../requests/#error-catchers). Additionally, other route parameters may be
necessary when building more interesting applications. The
[Requests](../requests) chapter, which follows this one, has further details on
routing and error handling.

! note: We prefer `#[macro_use]`, but you may prefer explicit imports.

  Throughout this guide and the majority of Rocket's documentation, we import
  `rocket` explicitly with `#[macro_use]` even though the Rust 2018 edition
  makes explicitly importing crates optional. However, explicitly importing with
  `#[macro_use]` imports macros globally, allowing you to use Rocket's macros
  anywhere in your application without importing them explicitly.

  You may instead prefer to import macros explicitly or refer to them with
  absolute paths: `use rocket::get;` or `#[rocket::get]`.

## Mounting

Before Rocket can dispatch requests to a route, the route needs to be _mounted_:

```rust
# #[macro_use] extern crate rocket;

# #[get("/world")]
# fn world() -> &'static str {
#     "hello, world!"
# }

rocket::build().mount("/hello", routes![world]);
```

The `mount` method takes as input:

   1. A _base_ path to namespace a list of routes under, here, `/hello`.
   2. A list of routes via the `routes!` macro: here, `routes![world]`, with
      multiple routes: `routes![a, b, c]`.

This creates a new `Rocket` instance via the `build` function and mounts the
`world` route to the `/hello` base path, making Rocket aware of the route.
`GET` requests to `/hello/world` will be directed to the `world` function.

The `mount` method, like all other builder methods on `Rocket`, can be chained
any number of times, and routes can be reused by mount points:

```rust
# #[macro_use] extern crate rocket;

# #[get("/world")]
# fn world() -> &'static str {
#     "hello, world!"
# }

rocket::build()
    .mount("/hello", routes![world])
    .mount("/hi", routes![world]);
```

By mounting `world` to both `/hello` and `/hi`, requests to `"/hello/world"`
_and_ `"/hi/world"` will be directed to the `world` function.

! note: In many cases, the base path will simply be `"/"`.

## Launching

Rocket begins serving requests after being _launched_, which starts a
multi-threaded asynchronous server and dispatches requests to matching routes as
they arrive.

There are two mechanisms by which a `Rocket` can be launched. The first and
preferred approach is via the `#[launch]` route attribute, which generates a
`main` function that sets up an async runtime and starts the server. With
`#[launch]`, our complete _Hello, world!_ application looks like:

```rust
#[macro_use] extern crate rocket;

#[get("/world")]
fn world() -> &'static str {
    "Hello, world!"
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/hello", routes![world])
}
```

Running the application, the console shows:

```sh
> cargo run
🔧 Configured for debug.
   >> address: 127.0.0.1
   >> port: 8000
   >> workers: [..]
   >> keep-alive: 5s
   >> limits: [..]
   >> tls: disabled
   >> temp dir: /tmp
   >> log level: normal
   >> cli colors: true
🛰  Routes:
   >> (world) GET /hello/world
🚀 Rocket has launched from http://127.0.0.1:8000
```

! tip: `#[launch]` infers the return type!

  Special to Rocket's `#[launch]` attribute, the return type of a function
  decorated with `#[launch]` is automatically inferred when the return type is
  set to `_`. If you prefer, you can also set the return type explicitly to
  `Rocket<Build>`.

If we visit `http://127.0.0.1:8000/hello/world`, we see `Hello, world!`, exactly
as we expected.

! note: This and other examples are on GitHub.

  An expanded version of this example's complete crate, ready to `cargo run`,
  can be found on [GitHub](@example/hello). You can find dozens of other
  complete examples, spanning all of Rocket's features, in the [GitHub examples
  directory](@example/).

The second approach uses the `#[rocket::main]` route attribute.
`#[rocket::main]` _also_ generates a `main` function that sets up an async
runtime but unlike `#[launch]`, allows _you_ to start the server:

```rust,no_run
# #[macro_use] extern crate rocket;
#
# #[get("/world")]
# fn world() -> &'static str {
#     "Hello, world!"
# }

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    rocket::build()
        .mount("/hello", routes![world])
        .launch()
        .await
}
```

`#[rocket::main]` is useful when a handle to the `Future` returned by `launch()`
is desired, or when the return value of [`launch()`] is to be inspected. The
[error handling example] for instance, inspects the return value.

[`launch()`]: @api/rocket/struct.Rocket.html#method.launch
[error handling example]: @example/error-handling

## Futures and Async

Rocket uses Rust [`Future`]s for concurrency. Asynchronous programming with
`Future`s and `async/await` allows route handlers to perform wait-heavy I/O such
as filesystem and network access while still allowing other requests to be make
progress. For an overview of Rust `Future`s, see [Asynchronous Programming in
Rust](https://rust-lang.github.io/async-book/).

In general, you should prefer to use async-ready libraries instead of
synchronous equivalents inside Rocket applications.

`async` appears in several places in Rocket:

* [Routes] and [Error Catchers] can be `async fn`s. Inside an `async fn`, you
  can `.await` `Future`s from Rocket or other libraries.
* Several of Rocket's traits, such as [`FromData`] and [`FromRequest`], have
  methods that return `Future`s.
* [`Data`] and [`DataStream`], incoming request data, and `Response` and `Body`,
  outgoing response data, are based on `tokio::io::AsyncRead` instead of
  `std::io::Read`.

You can find async-ready libraries on [crates.io](https://crates.io) with the
`async` tag.

[`Future`]: @std/future/trait.Future.html
[`Data`]: @api/rocket/struct.Data.html
[`DataStream`]: @api/rocket/data/struct.DataStream.html
[Routes]: ../requests
[Error Catchers]: ../requests#error-catchers
[`FromData`]: ../requests#body-data
[`FromRequest`]: ../requests#request-guards

! note

  Rocket v0.5 uses the tokio runtime. The runtime is started for you if you
  use `#[launch]` or `#[rocket::main]`, but you can still `launch()` a Rocket
  instance on a custom-built runtime by not using _either_ attribute.

### Async Routes

Rocket makes it easy to use `async/await` in routes.

```rust
# #[macro_use] extern crate rocket;
use rocket::tokio::time::{sleep, Duration};

#[get("/delay/<seconds>")]
async fn delay(seconds: u64) -> String {
    sleep(Duration::from_secs(seconds)).await;
    format!("Waited for {} seconds", seconds)
}
```

First, notice that the route function is an `async fn`. This enables the use of
`await` inside the handler. `sleep` is an asynchronous function, so we must
`await` it.

### Multitasking

Rust's `Future`s are a form of *cooperative multitasking*. In general, `Future`s
and `async fn`s should only `.await` on operations and never block.  Some common
examples of blocking include locking non-`async` mutexes, joining threads, or
using non-`async` library functions (including those in `std`) that perform I/O.

If a `Future` or `async fn` blocks the thread, inefficient resource usage,
stalls, or sometimes even deadlocks can occur.

Sometimes there is no good `async` alternative for a library or operation. If
necessary, you can convert a synchronous operation to an async one with
[`tokio::task::spawn_blocking`]:

```rust
# #[macro_use] extern crate rocket;
use std::io;

use rocket::tokio::task::spawn_blocking;

#[get("/blocking_task")]
async fn blocking_task() -> io::Result<Vec<u8>> {
    // In a real app, use rocket::fs::NamedFile or tokio::fs::File.
    let vec = spawn_blocking(|| std::fs::read("data.txt")).await
        .map_err(|e| io::Error::new(io::ErrorKind::Interrupted, e))??;

    Ok(vec)
}
```

[`tokio::task::spawn_blocking`]: @tokio/task/fn.spawn_blocking.html
