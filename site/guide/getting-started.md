# Getting Started

Let's create and run our first Rocket application. We'll ensure we have a
compatible version of Rust, create a new Cargo project that depends on Rocket,
and then run the application.

## Installing Rust

Rocket makes abundant use of Rust's syntax extensions and other advanced,
unstable features. Because of this, we'll need to use a supported _nightly_
version of Rust.

To install such a version, we recommend using `rustup`. Install `rustup` by
following the instructions on [its website](https://rustup.rs/). Once `rustup`
is installed, configure Rust nightly as your default toolchain by running the
command:

```sh
rustup default nightly-2017-12-21
```

If you prefer, once we setup a project directory in the following section, you
can use per-directory overrides to use the nightly version _only_ for your
Rocket project by running the following command in the directory:

```sh
rustup override set nightly-2017-12-21
```

### Minimum Nightly

Rocket generally requires the _latest_ version of Rust nightly. At present,
however, a Rocket dependency fails to build on the latest nightlies, temporarily
grounding Rocket (see [#513] for details). As a result, you'll need to use the
last known working nightly release while the issue is resolved. The commands
above already take this into account by installing the `2017-12-21` nightly.

Once the external issue has been resolved, you can upgrade to the latest nightly
with `rustup default nightly` or `rustup override set nightly`. If your Rocket
application suddenly stops building, it's likely that this issue has been
resolved. If Rocket fails to compile in the future, you should also ensure
you're using the latest version of Rust nightly and Rocket by updating your
toolchain and dependencies with:

```sh
rustup update && cargo update
```

[#513]: https://github.com/SergioBenitez/Rocket/issues/513

## Hello, world!

Let's write our first Rocket application! Start by creating a new binary-based
Cargo project and changing into the new directory:

```sh
cargo new hello-rocket --bin
cd hello-rocket
```

Now, add Rocket and its code generation facilities as dependencies of your
project by ensuring your `Cargo.toml` contains the following:

```
[dependencies]
rocket = "0.4.0-dev"
rocket_codegen = "0.4.0-dev"
```

Modify `src/main.rs` so that it contains the code for the Rocket `Hello, world!`
program, reproduced below:

```rust
#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate rocket;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

fn main() {
    rocket::ignite().mount("/", routes![index]).launch();
}
```

We won't explain exactly what the program does now; we leave that for the rest
of the guide. In short, it creates an `index` route, _mounts_ the route at the
`/` path, and launches the application. Compile and run the program with `cargo
run`. You should see the following:

```sh
🔧  Configured for development.
    => address: localhost
    => port: 54321
    => log: normal
    => workers: [core count * 2]
    => secret key: generated
    => limits: forms = 32KiB
    => tls: disabled
🛰  Mounting '/':
    => GET /
🚀  Rocket has launched from http://localhost:54321
```

Visit `http://localhost:54321` to see your first Rocket application in action!
