# Testing

Every application should be well tested. Rocket provides the tools to perform
unit and integration tests on your application as well as inspect Rocket
generated code.

## Tests

Rocket includes a built-in [testing](https://api.rocket.rs/rocket/testing/)
module that allows you to unit and integration test your Rocket applications.
Testing is simple:

  1. Construct a `Rocket` instance.
  2. Construct a `MockRequest`.
  3. Dispatch the request using the `Rocket` instance.
  4. Inspect, validate, and verify the `Response`.

After setting up, we'll walk through each of these steps for the "Hello, world!"
program below:

```rust
#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

#[get("/")]
fn hello() -> &'static str {
    "Hello, world!"
}
```

### Setting Up

For the `testing` module to be available, Rocket needs to be compiled with the
_testing_ feature enabled. Since this feature should only be enabled when your
application is compiled for testing, the recommended way to enable the _testing_
feature is via Cargo's `[dev-dependencies]` section in the `Cargo.toml` file as
follows:

```toml
[dev-dependencies]
rocket = { version = "0.2.6", features = ["testing"] }
```

With this in place, running `cargo test` will result in Cargo compiling Rocket
with the _testing_ feature, thus enabling the `testing` module.

You'll also need a `test` module with the proper imports:

```rust
#[cfg(test)]
mod test {
    use super::rocket;
    use rocket::testing::MockRequest;
    use rocket::http::{Status, Method};

    #[test]
    fn hello_world() {
        ...
    }
}
```

In the remainder of this section, we'll work on filling in the `hello_world`
testing function to ensure that the `hello` route results in a `Response` with
_"Hello, world!"_ in the body.

### Testing

We'll begin by constructing a `Rocket` instance with the `hello` route mounted
at the root path. We do this in the same way we would normally with one
exception: we need to refer to the `testing` route in the `super` namespace:

```rust
let rocket = rocket::ignite().mount("/", routes![super::hello]);
```

Next, we create a `MockRequest` that issues a `Get` request to the `"/"` path:

```rust
let mut req = MockRequest::new(Method::Get, "/");
```

We now ask Rocket to perform a full dispatch, which includes routing,
pre-processing and post-processing, and retrieve the `Response`:

```rust
let mut response = req.dispatch_with(&rocket);
```

Finally, we can test the
[Response](https://api.rocket.rs/rocket/struct.Response.html) values to ensure
that it contains the information we expect it to. We want to ensure two things:

  1. The status is `200 OK`.
  2. The body is the string "Hello, world!".

We do this by querying the `Response` object directly:

```rust
assert_eq!(response.status(), Status::Ok);

let body_str = response.body().and_then(|b| b.into_string());
assert_eq!(body_str, Some("Hello, world!".to_string()));
```

That's it! Run the tests with `cargo test`. The complete application, with
testing, can be found in the [GitHub testing
example](https://github.com/SergioBenitez/Rocket/tree/v0.2.6/examples/testing).

## Codegen Debug

It is sometimes useful to inspect the code that Rocket's code generation is
emitting, especially when you get a strange type error. To have Rocket log the
code that it is emitting to the console, set the `ROCKET_CODEGEN_DEBUG`
environment variable when compiling:

```rust
ROCKET_CODEGEN_DEBUG=1 cargo build
```

During compilation, you should see output like this:

```rust
Emitting item:
fn rocket_route_fn_hello<'_b>(_req: &'_b ::rocket::Request,
                              _data: ::rocket::Data)
                              -> ::rocket::handler::Outcome<'_b> {
    let responder = hello();
    ::rocket::handler::Outcome::of(responder)
}
```

This corresponds to the facade request handler Rocket generated for the `hello`
route.
