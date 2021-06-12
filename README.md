# Rocket

[![Build Status](https://github.com/SergioBenitez/Rocket/workflows/CI/badge.svg)](https://github.com/SergioBenitez/Rocket/actions)
[![Rocket Homepage](https://img.shields.io/badge/web-rocket.rs-red.svg?style=flat&label=https&colorB=d33847)](https://rocket.rs)
[![Current Crates.io Version](https://img.shields.io/crates/v/rocket.svg)](https://crates.io/crates/rocket)
[![Matrix: #rocket:mozilla.org](https://img.shields.io/badge/style-%23rocket:mozilla.org-blue.svg?style=flat&label=[m])](https://chat.mozilla.org/#/room/#rocket:mozilla.org)
[![IRC: #rocket on irc.libera.chat](https://img.shields.io/badge/style-%23rocket-blue.svg?style=flat&label=Libera.Chat)](https://kiwiirc.com/client/irc.libera.chat/#rocket)

Rocket is an async web framework for Rust with a focus on usability, security,
extensibility, and speed.

```rust
#[macro_use] extern crate rocket;

#[get("/<name>/<age>")]
fn hello(name: &str, age: u8) -> String {
    format!("Hello, {} year old named {}!", age, name)
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/hello", routes![hello])
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
[Overview]: https://rocket.rs/overview/
[Guide]: https://rocket.rs/guide/
[API Documentation]: https://api.rocket.rs/rocket/

The official community support channels are [`#rocket:mozilla.org`] on Matrix
and the bridged [`#rocket`] IRC channel on Libera.Chat at `irc.libera.chat`. We
recommend joining us on [Matrix via Element]. If your prefer IRC, you can join
via the [Kiwi IRC client] or a client of your own.

[`#rocket:mozilla.org`]: https://chat.mozilla.org/#/room/#rocket:mozilla.org
[`#rocket`]: https://kiwiirc.com/client/irc.libera.chat/#rocket
[Matrix via Element]: https://chat.mozilla.org/#/room/#rocket:mozilla.org
[Kiwi IRC Client]: https://kiwiirc.com/client/irc.libera.chat/#rocket

## Examples

An extensive number of examples are provided in the `examples/` directory. Each
example can be compiled and run with Cargo. For instance, the following sequence
of commands builds and runs the `Hello, world!` example:

```sh
cd examples/hello
cargo run
```

You should see `Hello, world!` by visiting `http://localhost:8000`.

## Building and Testing

The `core` directory contains the three core libraries: `lib`, `codegen`, and
`http` published as `rocket`, `rocket_codegen` and `rocket_http`, respectively.
The latter two are implementations details and are reexported from `rocket`.

### Testing

Rocket's complete test suite can be run with `./scripts/test.sh` from the root
of the source tree. The script builds and tests all libraries and examples in
all configurations. It accepts the following flags:

  * `--examples`: tests all examples in `examples/`
  * `--contrib`: tests each `contrib` library and feature individually
  * `--core`: tests each `core/lib` feature individually
  * `--benchmarks`: runs all benchmarks
  * `--all`: runs all tests in all configurations

Additionally, a `+${toolchain}` flag, where `${toolchain}` is a valid `rustup`
toolchain string, can be passed as the first parameter. The flag is forwarded to
`cargo` commands. Any other extra parameters are passed directly to `cargo`.

To test crates individually, simply run `cargo test --all-features` in the
crate's directory.

### Codegen Testing

Code generation diagnostics are tested using [`trybuild`]; tests can be found in
the `codegen/tests/ui-fail` directories of respective `codegen` crates. Each
test is symlinked into sibling `ui-fail-stable` and `ui-fail-nightly`
directories which contain the expected error output for stable and nightly
compilers, respectively. To update codegen test UI output, run a codegen test
suite with `TRYBUILD=overwrite` and inspect the `diff` of `.std*` files.

[`trybuild`]: https://docs.rs/trybuild/1

## Documentation

API documentation is built with `./scripts/mk-docs.sh`. The resulting assets are
uploaded to [api.rocket.rs](https://api.rocket.rs/).

Documentation for a released version `${x}` can be found at
`https://api.rocket.rs/v${x}` and `https://rocket.rs/v${x}`. For instance, the
documentation for `0.4` can be found at https://api.rocket.rs/v0.4 and
https://rocket.rs/v0.4. Documentation for unreleased versions in branch
`${branch}` be found at `https://api.rocket.rs/${branch}` and
`https://rocket.rs/${branch}`. For instance, the documentation for the `master`
branch can be found at https://api.rocket.rs/master and
https://rocket.rs/master. Documentation for unreleased branches is updated
periodically.

## Contributing

Contributions are absolutely, positively welcome and encouraged! Contributions
come in many forms. You could:

  1. Submit a feature request or bug report as an [issue].
  2. Ask for improved documentation as an [issue].
  3. Comment on [issues that require feedback].
  4. Contribute code via [pull requests].

[issue]: https://github.com/SergioBenitez/Rocket/issues
[issues that require feedback]: https://github.com/SergioBenitez/Rocket/issues?q=is%3Aissue+is%3Aopen+label%3A%22feedback+wanted%22
[pull requests]: https://github.com/SergioBenitez/Rocket/pulls

We aim to keep Rocket's code quality at the highest level. This means that any
code you contribute must be:

  * **Commented:** Complex and non-obvious functionality must be properly
    commented.
  * **Documented:** Public items _must_ have doc comments with examples, if
    applicable.
  * **Styled:** Your code's style should match the existing and surrounding code
    style.
  * **Simple:** Your code should accomplish its task as simply and
     idiomatically as possible.
  * **Tested:** You must write (and pass) convincing tests for any new
    functionality.
  * **Focused:** Your code should do what it's supposed to and nothing more.

All pull requests are code reviewed and tested by the CI. Note that unless you
explicitly state otherwise, any contribution intentionally submitted for
inclusion in Rocket by you shall be dual licensed under the MIT License and
Apache License, Version 2.0, without any additional terms or conditions.

## License

Rocket is licensed under either of the following, at your option:

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
 * MIT License ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)

The Rocket website source is licensed under [separate terms](site#license).
