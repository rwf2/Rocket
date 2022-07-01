# Quickstart

Before you can start writing a Rocket application, you'll need to install the
Rust toolchain. We recommend using [rustup](https://rustup.rs/). If you don't
have Rust installed and would like extra guidance doing so, see the [getting
started](../getting-started) section.

## Running Examples

The absolute fastest way to start experimenting with Rocket is to clone the
Rocket repository and run the included examples in the `examples/` directory.
For instance, the following set of commands runs the `hello` example:

```sh
git clone https://github.com/SergioBenitez/Rocket
cd Rocket
git checkout v0.5-rc
cd examples/hello
cargo run
```

There are numerous examples in the `examples/` directory. They can all be run
with `cargo run`.

! note

  The examples' `Cargo.toml` files will point to the locally cloned `rocket`
  libraries. When copying the examples for your own use, you should modify the
  `Cargo.toml` files as explained in the [Getting Started] guide.

[Getting Started]: ../getting-started
