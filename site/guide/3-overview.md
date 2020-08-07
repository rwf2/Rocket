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
  absolute paths: `use rocket::get;` or `#[rocket::get]`. The [`hello_2018`
  example](@example/hello_2018) showcases this alternative.

## Mounting

Before Rocket can dispatch requests to a route, the route needs to be _mounted_:

```rust
# #[macro_use] extern crate rocket;

# #[get("/world")]
# fn world() -> &'static str {
#     "hello, world!"
# }

fn main() {
    rocket::ignite().mount("/hello", routes![world]);
}
```

The `mount` method takes as input:

   1. A _base_ path to namespace a list of routes under, here, `"/hello"`.
   2. A list of routes via the `routes!` macro: here, `routes![world]`, with
      multiple routes: `routes![a, b, c]`.

This creates a new `Rocket` instance via the `ignite` function and mounts the
`world` route to the `"/hello"` path, making Rocket aware of the route. `GET`
requests to `"/hello/world"` will be directed to the `world` function.

! note: In many cases, the base path will simply be `"/"`.

### Namespacing

When a route is declared inside a module other than the root, you may find
yourself with unexpected errors when mounting:

```rust,compile_fail
# #[macro_use] extern crate rocket;

mod other {
    #[get("/world")]
    pub fn world() -> &'static str {
        "Hello, world!"
    }
}

#[get("/hello")]
pub fn hello() -> &'static str {
    "Hello, outside world!"
}

use other::world;

fn main() {
    // error[E0425]: cannot find value `static_rocket_route_info_for_world` in this scope
    rocket::ignite().mount("/hello", routes![hello, world]);
}
```

This occurs because the `routes!` macro implicitly converts the route's name
into the name of a structure generated by Rocket's code generation. The solution
is to refer to the route using a namespaced path instead:

```rust
# #[macro_use] extern crate rocket;

# #[get("/")] pub fn hello() {}
# mod other { #[get("/world")] pub fn world() {} }

rocket::ignite().mount("/hello", routes![hello, other::world]);
```

## Launching

Now that Rocket knows about the route, you can tell Rocket to start accepting
requests via the `launch` method. The method starts up the server and waits for
incoming requests. When a request arrives, Rocket finds the matching route and
dispatches the request to the route's handler.

We typically use `#[launch]`, which generates a `main` function.
Our complete _Hello, world!_ application thus looks like:

```rust
#[macro_use] extern crate rocket;

#[get("/world")]
fn world() -> &'static str {
    "Hello, world!"
}

#[launch]
fn rocket() -> rocket::Rocket {
    rocket::ignite().mount("/hello", routes![world])
}
```

We've imported the `rocket` crate and all of its macros into our namespace via
`#[macro_use] extern crate rocket`. Finally, we call the `launch` method in the
`main` function.

Running the application, the console shows:

```sh
🔧  Configured for development.
    => address: localhost
    => port: 8000
    => log: normal
    => workers: [logical cores * 2]
    => secret key: generated
    => limits: forms = 32KiB
    => keep-alive: 5s
    => tls: disabled
🛰  Mounting '/hello':
    => GET /hello/world (world)
🚀  Rocket has launched from http://localhost:8000
```

If we visit `localhost:8000/hello/world`, we see `Hello, world!`, exactly as we
expected.

A version of this example's complete crate, ready to `cargo run`, can be found
on [GitHub](@example/hello_world). You can find dozens of other complete
examples, spanning all of Rocket's features, in the [GitHub examples
directory](@example/).

## Futures and Async

Rocket uses Rust `Future`s for concurrency. Asynchronous programming with
`Future`s and `async/await` allows route handlers to perform wait-heavy I/O such
as filesystem and network access while still allowing other requests to be
processed.  For an overview of Rust `Future`s, see [Asynchronous Programming in
Rust](https://rust-lang.github.io/async-book/).

In general, you should prefer to use async-ready libraries instead of
synchronous equivalents inside Rocket applications.

`async` appears in several places in Rocket:

* [Routes](../requests) and [Error Catchers](../requests#error-catchers) can be
  `async fn`s. Inside an `async fn`, you can `.await` `Future`s from Rocket or
  other libraries
* Several of Rocket's traits, such as [`FromData`](../requests#body-data) and
  [`FromRequest`](../requests#request-guards), have methods that return
  `Future`s.
* `Data` and `DataStream` (incoming request data) and `Response` and `Body`
  (outgoing response data) are based on `tokio::io::AsyncRead` instead of
  `std::io::Read`.

You can find async-ready libraries on [crates.io](https://crates.io) with the
`async` tag.

! note

  Rocket 0.5 uses the tokio (0.2) runtime. The runtime is started for you if you
  use `#[launch]` or `#[rocket::main]`, but you can still `launch()` a
  rocket instance on a custom-built `Runtime`.

### Cooperative Multitasking

Rust's `Future`s are a form of *cooperative multitasking*. In general, `Future`s
and `async fn`s should only `.await` on other operations and never block.  Some
common examples of blocking include locking mutexes, joining threads, or using
non-`async` library functions (including those in `std`) that perform I/O.

If a `Future` or `async fn` blocks the thread, inefficient resource usage,
stalls, or sometimes even deadlocks can occur.

! note

  Sometimes there is no good async alternative for a library or operation. If
  necessary, you can convert a synchronous operation to an async one with
  `tokio::task::spawn_blocking`:

  ```rust
  # #[macro_use] extern crate rocket;
  use std::io;
  use rocket::tokio::task::spawn_blocking;
  use rocket::response::Debug;

  #[get("/blocking_task")]
  async fn blocking_task() -> Result<Vec<u8>, Debug<io::Error>> {
      // In a real app, we'd use rocket::response::NamedFile or tokio::fs::File.
      let io_result = spawn_blocking(|| std::fs::read("data.txt")).await
        .map_err(|join_err| io::Error::new(io::ErrorKind::Interrupted, join_err))?;

      Ok(io_result?)
  }
  ```
