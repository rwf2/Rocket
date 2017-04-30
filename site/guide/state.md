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
[manage](https://api.rocket.rs/rocket/struct.Rocket.html#method.manage) method
on a `Rocket` instance. For example, to ask Rocket to manage a `HitCount`
structure with an internal `AtomicUsize` with an initial value of `0`, we can
write the following:

```rust
struct HitCount(AtomicUsize);

rocket::ignite().manage(HitCount(AtomicUsize::new(0)));
```

The `manage` method can be called any number of times as long as each call
refers to a value of a different type. For instance, to have Rocket manage both
a `HitCount` value and a `Config` value, we can write:

```rust
rocket::ignite()
  .manage(HitCount(AtomicUsize::new(0)))
  .manage(Config::from(user_input));
```

### Retrieving State

State that is being managed by Rocket can be retrieved via the
[State](https://api.rocket.rs/rocket/struct.State.html) type: a [request
guard](/guide/requests/#request-guards) for managed state. To use the request
guard, add a `State<T>` type to any request handler, where `T` is the
type of the managed state. For example, we can retrieve and respond with the
current `HitCount` in a `count` route as follows:

```rust
#[get("/count")]
fn count(hit_count: State<HitCount>) -> String {
    let current_count = hit_count.0.load(Ordering::Relaxed);
    format!("Number of visits: {}", current_count)
}
```

You can retrieve more than one `State` type in a single route as well:

```rust
#[get("/state")]
fn state(hit_count: State<HitCount>, config: State<Config>) -> T { ... }
```

It can also be useful to retrieve managed state from a `FromRequest`
implementation. To do so, invoke the `from_request` method of a `State<T>` type
directly, passing in the `req` parameter of `from_request`:

```rust
fn from_request(req: &'a Request<'r>) -> request::Outcome<T, ()> {
    let count = match <State<HitCount> as FromRequest>::from_request(req) {
        Outcome::Success(count) => count,
        ...
    };
    ...
}
```

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
    let current_count = hit_count.0.load(Ordering::Relaxed);
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
  |                 ^^^^^^^^^^^^^^^
  |
  = note: this State request guard will always fail
help: maybe add a call to 'manage' here?
 --> src/main.rs:8:5
  |
8 |     rocket::ignite()
  |     ^^^^^^^^^^^^^^^^
```

The `unmanaged_state` lint isn't perfect. In particular, it cannot track calls
to `manage` across function boundaries. You can disable the lint on a per-route
basis by adding `#[allow(unmanaged_state)]` to a route handler. If you wish to
disable the lint globally, add `#![allow(unmanaged_state)]` to your crate
attributes.

You can find a complete example using the `HitCounter` structure in the [state
example on
GitHub](https://github.com/SergioBenitez/Rocket/tree/v0.2.6/examples/state) and
learn more about the [manage
method](https://api.rocket.rs/rocket/struct.Rocket.html#method.manage) and
[State type](https://api.rocket.rs/rocket/struct.State.html) in the API docs.
