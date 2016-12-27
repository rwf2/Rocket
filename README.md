# Rocket [![Build Status](https://travis-ci.org/SergioBenitez/Rocket.svg?branch=master)](https://travis-ci.org/SergioBenitez/Rocket)

Rocket is web framework for Rust (nightly) with a focus on ease-of-use,
expressability, and speed. Here's an example of a complete Rocket application:

```rust
#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

#[get("/<name>/<age>")]
fn hello(name: &str, age: u8) -> String {
    format!("Hello, {} year old named {}!", age, name)
}

fn main() {
    rocket::ignite().mount("/hello", routes![hello]).launch();
}
```

Visiting `localhost:8000/hello/John/58`, for example, will trigger the `hello`
route resulting in the string `Hello, 58 year old named John!` being sent to the
browser. If an `<age>` string was passed in that can't be parsed as a `u8`, the
route won't get called, resulting in a 404 error.

## Documentation

Rocket is extensively documented:

  * [Overview]: A brief look at what makes Rocket special.
  * [Quickstart]: How to get started as quickly as possible.
  * [Getting Started]: How to start your first Rocket project.
  * [Guide]: A detailed guide and reference to Rocket.
  * [API Documentation]: The "rustdocs".

[Quickstart]: https://rocket.rs/guide/quickstart
[Getting Started]: https://rocket.rs/guide/getting-started
[Overview]: https://rocket.rs/overview
[Guide]: https://rocket.rs/guide
[API Documentation]: https://api.rocket.rs/rocket

## Building

### Nightly

Rocket requires a nightly version of Rust as it makes heavy use of syntax
extensions. This means that the first two unwieldly lines in the introductory
example above are required.

### Core, Codegen, and Contrib

All of the Rocket libraries are managed by Cargo. As a result, compiling them is
simple.

  * Core: `cd lib && cargo build`
  * Codegen: `cd codegen && cargo build`
  * Contrib: `cd contrib && cargo build --all-features`

### Examples

Rocket ships with an extensive number of examples in the `examples/` directory
which can be compiled and run with Cargo. For instance, the following sequence
of commands builds and runs the `Hello, world!` example:

```
cd examples/hello_world
cargo run
```

You should see `Hello, world!` by visiting `http://localhost:8000`.

## Testing

To test Rocket, simply run `./scripts/test.sh` from the root of the source tree.
This will build and test the `core`, `codegen`, and `contrib` libraries as well
as all of the examples. This is the script that gets run by Travis CI. To test a
crate individually, run `cargo test --all-features`.

### Core

Testing for the core library is done inline in the corresponding module. For
example, the tests for routing can be found at the bottom of the
`lib/src/router/mod.rs` file.

### Codegen

Code generation tests can be found in `codegen/tests`. We use the
[compiletest](https://crates.io/crates/compiletest_rs) library, which was
extracted from `rustc`, for testing. See the [compiler test
documentation](https://github.com/rust-lang/rust/blob/master/COMPILER_TESTS.md)
for information on how to write compiler tests.

## Documentation

You can build the Rocket API documentation locally by running
`./scripts/mk-docs.sh`. The resulting documentation is what gets uploaded to
[api.rocket.rs](https://api.rocket.rs).

## Contributing

Contributions are absolutely, positively welcome and encouraged! Contributions
come in many forms. You could:

  1. Submit a feature request or bug report as an [issue](https://github.com/SergioBenitez/Rocket/issues).
  2. Ask for improved documentation as an [issue](https://github.com/SergioBenitez/Rocket/issues).
  3. Comment on [issues that require
     feedback](https://github.com/SergioBenitez/Rocket/issues?q=is%3Aissue+is%3Aopen+label%3A%22feedback+wanted%22).
  4. Contribute code via [pull requests](https://github.com/SergioBenitez/Rocket/pulls).

We aim to keep Rocket's code quality at the highest level. This means that any
code you contribute must be:

  * **Commented:** Public items _must_ be commented.
  * **Documented:** Exposed items _must_ have rustdoc comments with
    examples, if applicable.
  * **Styled:** Your code should be `rustfmt`'d when possible.
  * **Simple:** Your code should accomplish its task as simply and
     idiomatically as possible.
  * **Tested:** You must add (and pass) convincing tests for any functionality you add.
  * **Focused:** Your code should do what it's supposed to do and nothing more.

All pull requests are code reviewed and tested by the CI. Note that unless you
explicitly state otherwise, any contribution intentionally submitted for
inclusion in Rocket by you shall be dual licensed under the MIT License and
Apache License, Version 2.0, without any additional terms or conditions.

## Performance

Rocket is designed to be performant. At this time, its performance is
[bottlenecked by the Hyper HTTP
library](https://github.com/SergioBenitez/Rocket/issues/17). Even so, Rocket
currently performs _better_ than the latest version of Hyper on a simple "Hello,
world!" benchmark:

**Machine Specs:**

  * **Logical Cores:** 12 (6 cores x 2 threads)
  * **Memory:** 24gb ECC DDR3 @ 1600mhz
  * **Processor:** Intel Xeon X5675 @ 3.07GHz
  * **Operating System:** Mac OS X v10.11.6

**Hyper v0.10.0-a.0** (46 LOC) results (best of 3, +/- 2000 req/s, +/- 10us latency):

    Running 10s test @ http://localhost:80
      2 threads and 10 connections
      Thread Stats   Avg      Stdev     Max   +/- Stdev
        Latency   175.12us   40.38us 429.00us   70.79%
        Req/Sec    28.00k     2.41k   36.79k    72.28%
      562692 requests in 10.10s, 81.57MB read
    Requests/sec:  55715.98
    Transfer/sec:      8.08MB

**Rocket v0.1.0** (8 LOC) results (best of 3, +/- 1000 req/s, +/- 5us latency):

    Running 10s test @ http://localhost:80
      2 threads and 10 connections
      Thread Stats   Avg      Stdev     Max   +/- Stdev
        Latency   161.33us   37.40us   2.08ms   75.89%
        Req/Sec    30.10k     1.13k   33.28k    72.77%
      604782 requests in 10.10s, 84.21MB read
    Requests/sec:  59883.30
    Transfer/sec:      8.34MB

**Summary:**

  * Rocket throughput higher by 7.5% (higher is better).
  * Rocket latency lower by 7.8% (lower is better).

### Future Improvements

Rocket is currently built on a synchronous HTTP backend. Once the Rust
asynchronous I/O libraries have stabilized, a migration to a new, more
performant HTTP backend is planned. We expect performance to improve
significantly at that time. The [Stabilize HTTP
Library](https://github.com/SergioBenitez/Rocket/issues/17) issue tracks the
progress on this front.

## License

Rocket is licensed under either of the following, at your option:

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
