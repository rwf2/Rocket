# Fairings

Fairings are Rocket's approach to structured middleware. With fairings, your
application can hook into the request lifecycle to record or rewrite information
about incoming requests and outgoing responses.

## Overview

Any type that implements the [`Fairing`] trait is a _fairing_. Fairings hook
into Rocket's request lifecycle, receiving callbacks for events such as incoming
requests and outgoing responses. Rocket passes information about these events to
the fairing, and the fairing can do what it wants with the information. This
includes rewriting data when applicable, recording information about the event
or data, or doing nothing at all.

Rocket’s fairings are a lot like middleware from other frameworks, but they bear
a few key distinctions:

  * Fairings **cannot** terminate or respond to an incoming request directly.
  * Fairings **cannot** inject arbitrary, non-request data into a request.
  * Fairings _can_ prevent an application from launching.
  * Fairings _can_ inspect and modify the application's configuration.

If you are familiar with middleware from other frameworks, you may find yourself
reaching for fairings instinctively. Before doing so, remember that Rocket
provides a rich set of mechanisms such as [request guards] and [data guards]
that can be used to solve problems in a clean, composable, and robust manner.

As a general rule of thumb, only _globally applicable_ actions should be
effected through fairings. You should _not_ use a fairing to implement
authentication or authorization (preferring to use a [request guard] instead)
_unless_ the authentication or authorization applies to all or most of the
application. On the other hand, you _should_ use a fairing to record timing and
usage statistics or to enforce global security policies.

[`Fairing`]: https://api.rocket.rs/rocket/fairing/trait.Fairing.html
[request guard]: /guide/requests/#request-guards
[request guards]: /guide/requests/#request-guards
[data guards]: /guide/requests/#body-data

### Attaching

Fairings are registered with Rocket via the [`attach`] method on a [`Rocket`]
instance. Only when a fairing is attached will its callbacks fire. As an
example, the following snippet attached two fairings,  `req_fairing` and
`res_fairing`, to a new Rocket instance:

```rust
rocket::ignite()
    .attach(req_fairing)
    .attach(res_fairing)
    .launch();
```

[`attach`]: https://api.rocket.rs/rocket/struct.Rocket.html#method.attach
[`Rocket`]: https://api.rocket.rs/rocket/struct.Rocket.html

Fairings are executed in the order in which they are attached: the first
attached fairing has its callbacks executed before all others. Because fairing
callbacks may not be commutative, the order in which fairings are attached may
be significant.

### Callbacks

There are four events for which Rocket issues fairing callbacks. Each of these
events is described below:

  * **Attach (`on_attach`)**

    An attach callback is called when a fairing is first attached via the
    [`attach`](https://api.rocket.rs/rocket/struct.Rocket.html#method.attach)
    method. An attach callback can arbitrarily modify the `Rocket` instance
    being constructed and optionally abort launch. Attach fairings are commonly
    used to parse and validate configuration values, aborting on bad
    configurations, and inserting the parsed value into managed state for later
    retrieval.

  * **Launch (`on_launch`)**

    A launch callback is called immediately before the Rocket application has
    launched. A launch callback can inspect the `Rocket` instance being
    launched. A launch callback can be a convenient hook for launching services
    related to the Rocket application being launched.

  * **Request (`on_request`)**

    A request callback is called just after a request is received. A request
    callback can modify the request at will and peek into the incoming data. It
    may not, however, abort or respond directly to the request; these issues are
    better handled via request guards or via response callbacks.

  * **Response (`on_response`)**

    A response callback is called when a response is ready to be sent to the
    client. A response callback can modify part or all of the response. As such,
    a response fairing can be used to provide a response when the greater
    applications fails to by rewriting **404** responses as desired. As another
    example, response fairings can also be used to inject headers into all
    outgoing responses.

## Implementing

Recall that a fairing is any type that implements the [`Fairing`] trait. A
`Fairing` implementation has one required method: [`info`], which returns an
[`Info`] structure. This structure is used by Rocket to assign a name to the
fairing and determine the set of callbacks the fairing is registering for. A
`Fairing` can implement any of the available callbacks: [`on_attach`],
[`on_launch`], [`on_request`], and [`on_response`]. Each callback has a default
implementation that does absolutely nothing.

[`Info`]: https://api.rocket.rs/rocket/fairing/struct.Info.html
[`info`]: https://api.rocket.rs/rocket/fairing/trait.Fairing.html#tymethod.info
[`on_attach`]: https://api.rocket.rs/rocket/fairing/trait.Fairing.html#method.on_attach
[`on_launch`]: https://api.rocket.rs/rocket/fairing/trait.Fairing.html#method.on_launch
[`on_request`]: https://api.rocket.rs/rocket/fairing/trait.Fairing.html#method.on_request
[`on_response`]: https://api.rocket.rs/rocket/fairing/trait.Fairing.html#method.on_response

### Requirements

A type implementing `Fairing` is required to be `Send + Sync + 'static`. This
means that the fairing must be sendable across thread boundaries (`Send`),
thread-safe (`Sync`), and have only static references, if any (`'static`). Note
that these bounds _do not_ prohibit a `Fairing` from holding state: the state
need simply be thread-safe and statically available or heap allocated.

### Example

Imagine that we want to record the number of `GET` and `POST` requests that our
application has received. While we could do this with request guards and managed
state, it would require us to annotate every `GET` and `POST` request with
custom types, polluting handler signatures. Instead, we can create a simple
fairing that acts globally.

The code for a `Counter` fairing below implements exactly this. The fairing
receives a request callback, where it increments a counter on each `GET` and
`POST` request. It also receives a response callback, where it responds to
unrouted requests to the `/counts` path by returning the recorded number of
counts.

```rust
struct Counter {
    get: AtomicUsize,
    post: AtomicUsize,
}

impl Fairing for Counter {
    // This is a request and response fairing named "GET/POST Counter".
    fn info(&self) -> Info {
        Info {
            name: "GET/POST Counter",
            kind: Kind::Request | Kind::Response
        }
    }

    // Increment the counter for `GET` and `POST` requests.
    fn on_request(&self, request: &mut Request, _: &Data) {
        match request.method() {
            Method::Get => self.get.fetch_add(1, Ordering::Relaxed),
            Method::Post => self.post.fetch_add(1, Ordering::Relaxed),
            _ => return
        }
    }

    fn on_response(&self, request: &Request, response: &mut Response) {
        // Don't change a successful user's response, ever.
        if response.status() != Status::NotFound {
            return
        }

        // Rewrite the response to return the current counts.
        if request.method() == Method::Get && request.uri().path() == "/counts" {
            let get_count = self.get.load(Ordering::Relaxed);
            let post_count = self.post.load(Ordering::Relaxed);
            let body = format!("Get: {}\nPost: {}", get_count, post_count);

            response.set_status(Status::Ok);
            response.set_header(ContentType::Plain);
            response.set_sized_body(Cursor::new(body));
        }
    }
}
```

For brevity, imports are not shown. The complete example can be found in the
[`Fairing`
documentation](https://api.rocket.rs/rocket/fairing/trait.Fairing.html#example).

## Ad-Hoc Fairings

For simple occasions, implementing the `Fairing` trait can be cumbersome. This
is why Rocket provides the [`AdHoc`] type, which creates a fairing from a simple
function or closure. Using the `AdHoc` type is easy: simply call the
`on_attach`, `on_launch`, `on_request`, or `on_response` constructors on `AdHoc`
to create an `AdHoc` structure from a function or closure.

As an example, the code below creates a `Rocket` instance with two attached
ad-hoc fairings. The first, a launch fairing, simply prints a message indicating
that the application is about to the launch. The second, a request fairing,
changes the method of all requests to `PUT`.

```rust
use rocket::fairing::AdHoc;
use rocket::http::Method;

rocket::ignite()
    .attach(AdHoc::on_launch(|_| {
        println!("Rocket is about to launch! Exciting!");
    }))
    .attach(AdHoc::on_request(|req, _| {
        req.set_method(Method::Put);
    }));
```

[`AdHoc`]: https://api.rocket.rs/rocket/fairing/enum.AdHoc.html

