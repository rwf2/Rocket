# Getting Started

Let's create and run our first Rocket application. We'll ensure we have a
compatible version of Rust, create a new Cargo project that depends on Rocket,
and then run the application.

## Installing Rust

Rocket makes abundant use of Rust's syntax extensions and other advanced,
unstable features. Because of this, we'll need to use a nightly version of Rust.
If you already have a working installation of the latest Rust nightly, feel free
to skip to the next section.

To install a nightly version of Rust, we recommend using `rustup`. Install
`rustup` by following the instructions on [its website](https://rustup.rs/).
Once `rustup` is installed, configure Rust nightly as your default toolchain by
running the command:

```sh
rustup default nightly
```

If you prefer, once we setup a project directory in the following section, you
can use per-directory overrides to use the nightly version _only_ for your
Rocket project by running the following command in the directory:

```sh
rustup override set nightly
```

! warning: Rocket requires the _latest_ version of Rust nightly.

  If your Rocket application suddenly stops building, ensure you're using the
  latest version of Rust nightly and Rocket by updating your toolchain and
  dependencies with:

  `rustup update && cargo update`

## Hello, world!

Let's write our first Rocket application! Start by creating a new binary-based
Cargo project and changing into the new directory:

```sh
cargo new hello-rocket --bin
cd hello-rocket
```

Now, add Rocket as a dependency in your `Cargo.toml`:

```
[dependencies]
rocket = "0.4.0-dev"
```

Modify `src/main.rs` so that it contains the code for the Rocket `Hello, world!`
program, reproduced below:

```rust
#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

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
    => port: 8000
    => log: normal
    => workers: 24
    => secret key: generated
    => limits: forms = 32KiB
    => keep-alive: 5s
    => tls: disabled
🛰  Mounting '/':
    => GET / (hello)
🚀  Rocket has launched from http://localhost:8000
```

Visit `http://localhost:8000` to see your first Rocket application in action!

! tip: Don't like colors or emoji?

  You can disable colors and emoji by setting the `ROCKET_CLI_COLORS`
  environment variable to `0` or `off` when running a Rocket binary:

  `ROCKET_CLI_COLORS=off cargo run`
