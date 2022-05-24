#![recursion_limit="256"]

#![doc(html_root_url = "https://api.rocket.rs/v0.5-rc")]
#![doc(html_favicon_url = "https://rocket.rs/images/favicon.ico")]
#![doc(html_logo_url = "https://rocket.rs/images/logo-boxed.png")]
#![cfg_attr(nightly, feature(doc_cfg))]
#![cfg_attr(nightly, feature(decl_macro))]

#![warn(rust_2018_idioms)]
#![warn(missing_docs)]

//! # Rocket - Core API Documentation
//!
//! Hello, and welcome to the core Rocket API documentation!
//!
//! This API documentation is highly technical and is purely a reference.
//! There's an [overview] of Rocket on the main site as well as a [full,
//! detailed guide]. If you'd like pointers on getting started, see the
//! [quickstart] or [getting started] chapters of the guide.
//!
//! [overview]: https://rocket.rs/v0.5-rc/overview
//! [full, detailed guide]: https://rocket.rs/v0.5-rc/guide
//! [quickstart]: https://rocket.rs/v0.5-rc/guide/quickstart
//! [getting started]: https://rocket.rs/v0.5-rc/guide/getting-started
//!
//! ## Usage
//!
//! Depend on `rocket` in `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! rocket = "0.5.0-rc.2"
//! ```
//!
//! <small>Note that development versions, tagged with `-dev`, are not published
//! and need to be specified as [git dependencies].</small>
//!
//! See the [guide](https://rocket.rs/v0.5-rc/guide) for more information on how
//! to write Rocket applications. Here's a simple example to get you started:
//!
//! [git dependencies]: https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#specifying-dependencies-from-git-repositories
//!
//! ```rust,no_run
//! #[macro_use] extern crate rocket;
//!
//! #[get("/")]
//! fn hello() -> &'static str {
//!     "Hello, world!"
//! }
//!
//! #[launch]
//! fn rocket() -> _ {
//!     rocket::build().mount("/", routes![hello])
//! }
//! ```
//!
//! ## Features
//!
//! To avoid compiling unused dependencies, Rocket gates certain features. With
//! the exception of `http2`, all are disabled by default:
//!
//! | Feature   | Description                                             |
//! |-----------|---------------------------------------------------------|
//! | `secrets` | Support for authenticated, encrypted [private cookies]. |
//! | `tls`     | Support for [TLS] encrypted connections.                |
//! | `mtls`    | Support for verified clients via [mutual TLS].          |
//! | `http2`   | Support for HTTP/2 (enabled by default).                |
//! | `json`    | Support for [JSON (de)serialization].                   |
//! | `msgpack` | Support for [MessagePack (de)serialization].            |
//! | `uuid`    | Support for [UUID value parsing and (de)serialization]. |
//!
//! Disabled features can be selectively enabled in `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! rocket = { version = "0.5.0-rc.2", features = ["secrets", "tls", "json"] }
//! ```
//!
//! Conversely, HTTP/2 can be disabled:
//!
//! ```toml
//! [dependencies]
//! rocket = { version = "0.5.0-rc.2", default-features = false }
//! ```
//!
//! [JSON (de)serialization]: crate::serde::json
//! [MessagePack (de)serialization]: crate::serde::msgpack
//! [UUID value parsing and (de)serialization]: crate::serde::uuid
//! [private cookies]: https://rocket.rs/v0.5-rc/guide/requests/#private-cookies
//! [TLS]: https://rocket.rs/v0.5-rc/guide/configuration/#tls
//! [mutual TLS]: crate::mtls
//!
//! ## Configuration
//!
//! Rocket offers a rich, extensible configuration system built on [Figment]. By
//! default, Rocket applications are configured via a `Rocket.toml` file
//! and/or `ROCKET_{PARAM}` environment variables, but applications may
//! configure their own sources. See the [configuration guide] for full details.
//!
//! ## Testing
//!
//! The [`local`] module contains structures that facilitate unit and
//! integration testing of a Rocket application. The top-level [`local`] module
//! documentation and the [testing guide] include detailed examples.
//!
//! [configuration guide]: https://rocket.rs/v0.5-rc/guide/configuration/
//! [testing guide]: https://rocket.rs/v0.5-rc/guide/testing/#testing
//! [Figment]: https://docs.rs/figment

/// These are public dependencies! Update docs if these are changed, especially
/// figment's version number in docs.
#[doc(hidden)] pub use yansi;
#[doc(hidden)] pub use async_stream;
pub use futures;
pub use tokio;
pub use figment;
pub use time;

#[doc(hidden)]
#[macro_use] pub mod log;
#[macro_use] pub mod outcome;
#[macro_use] pub mod data;
#[doc(hidden)] pub mod sentinel;
pub mod local;
pub mod request;
pub mod response;
pub mod config;
pub mod form;
pub mod fairing;
pub mod error;
pub mod catcher;
pub mod route;
pub mod serde;
pub mod shield;
pub mod fs;

// Reexport of HTTP everything.
pub mod http {
    //! Types that map to concepts in HTTP.
    //!
    //! This module exports types that map to HTTP concepts or to the underlying
    //! HTTP library when needed.

    #[doc(inline)]
    pub use rocket_http::*;

    /// Re-exported hyper HTTP library types.
    ///
    /// All types that are re-exported from Hyper reside inside of this module.
    /// These types will, with certainty, be removed with time, but they reside here
    /// while necessary.
    pub mod hyper {
        #[doc(hidden)]
        pub use rocket_http::hyper::*;

        pub use rocket_http::hyper::header;
    }

    #[doc(inline)]
    pub use crate::cookies::*;
}

#[cfg(feature = "mtls")]
#[cfg_attr(nightly, doc(cfg(feature = "mtls")))]
pub mod mtls;

/// TODO: We need a futures mod or something.
mod trip_wire;
mod shutdown;
mod server;
mod ext;
mod state;
mod cookies;
mod rocket;
mod router;
mod phase;

#[doc(inline)] pub use crate::response::Response;
#[doc(inline)] pub use crate::data::Data;
#[doc(inline)] pub use crate::config::Config;
#[doc(inline)] pub use crate::catcher::Catcher;
#[doc(inline)] pub use crate::route::Route;
#[doc(hidden)] pub use either::Either;
#[doc(inline)] pub use phase::{Phase, Build, Ignite, Orbit};
#[doc(inline)] pub use error::Error;
#[doc(inline)] pub use sentinel::Sentinel;
#[doc(inline)] pub use rocket_codegen::*;
pub use crate::rocket::Rocket;
pub use crate::request::Request;
pub use crate::shutdown::Shutdown;
pub use crate::state::State;

/// Creates a [`Rocket`] instance with the default config provider: aliases
/// [`Rocket::build()`].
pub fn build() -> Rocket<Build> {
    Rocket::build()
}

/// Creates a [`Rocket`] instance with a custom config provider: aliases
/// [`Rocket::custom()`].
pub fn custom<T: figment::Provider>(provider: T) -> Rocket<Build> {
    Rocket::custom(provider)
}

/// Retrofits support for `async fn` in trait impls and declarations.
///
/// Any trait declaration or trait `impl` decorated with `#[async_trait]` is
/// retrofitted with support for `async fn`s:
///
/// ```rust
/// # use rocket::*;
/// #[async_trait]
/// trait MyAsyncTrait {
///     async fn do_async_work();
/// }
///
/// #[async_trait]
/// impl MyAsyncTrait for () {
///     async fn do_async_work() { /* .. */ }
/// }
/// ```
///
/// All `impl`s for a trait declared with `#[async_trait]` must themselves be
/// decorated with `#[async_trait]`. Many of Rocket's traits, such as
/// [`FromRequest`](crate::request::FromRequest) and
/// [`Fairing`](crate::fairing::Fairing) are `async`. As such, implementations
/// of said traits must be decorated with `#[async_trait]`. See the individual
/// trait docs for trait-specific details.
///
/// For more details on `#[async_trait]`, see [`async_trait`](mod@async_trait).
#[doc(inline)]
pub use async_trait::async_trait;

/// WARNING: This is unstable! Do not use this method outside of Rocket!
#[doc(hidden)]
pub fn async_run<F, R>(fut: F, workers: usize, sync: usize, force_end: bool, name: &str) -> R
    where F: std::future::Future<Output = R>
{
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .thread_name(name)
        .worker_threads(workers)
        .max_blocking_threads(sync)
        .enable_all()
        .build()
        .expect("create tokio runtime");

    let result = runtime.block_on(fut);
    if force_end {
        runtime.shutdown_timeout(std::time::Duration::from_millis(500));
    }

    result
}

/// WARNING: This is unstable! Do not use this method outside of Rocket!
#[doc(hidden)]
pub fn async_test<R>(fut: impl std::future::Future<Output = R>) -> R {
    async_run(fut, 1, 32, true, "rocket-worker-test-thread")
}

/// WARNING: This is unstable! Do not use this method outside of Rocket!
#[doc(hidden)]
pub fn async_main<R>(fut: impl std::future::Future<Output = R> + Send) -> R {
    // FIXME: These config values won't reflect swaps of `Rocket` in attach
    // fairings with different config values, or values from non-Rocket configs.
    // See tokio-rs/tokio#3329 for a necessary solution in `tokio`.
    let c = Config::from(Config::figment());
    async_run(fut, c.workers, c.max_blocking, c.shutdown.force, "rocket-worker-thread")
}

/// Executes a `future` to completion on a new tokio-based Rocket async runtime.
///
/// The runtime is terminated on shutdown, and the future's resolved value is
/// returned.
///
/// # Considerations
///
/// This function is a low-level mechanism intended to be used to execute the
/// future returned by [`Rocket::launch()`] in a self-contained async runtime
/// designed for Rocket. It runs futures in exactly the same manner as
/// [`#[launch]`](crate::launch) and [`#[main]`](crate::main) do and is thus
/// _never_ the preferred mechanism for running a Rocket application. _Always_
/// prefer to use the [`#[launch]`](crate::launch) or [`#[main]`](crate::main)
/// attributes. For example [`#[main]`](crate::main) can be used even when
/// Rocket is just a small part of a bigger application:
///
/// ```rust,no_run
/// #[rocket::main]
/// async fn main() {
///     # let should_start_server_in_foreground = false;
///     # let should_start_server_in_background = false;
///     let rocket = rocket::build();
///     if should_start_server_in_foreground {
///         rocket::build().launch().await;
///     } else if should_start_server_in_background {
///         rocket::tokio::spawn(rocket.launch());
///     } else {
///         // do something else
///     }
/// }
/// ```
///
/// See [Rocket#launching] for more on using these attributes.
///
/// # Example
///
/// Build an instance of Rocket, launch it, and wait for shutdown:
///
/// ```rust,no_run
/// use rocket::fairing::AdHoc;
///
/// let rocket = rocket::build()
///     .attach(AdHoc::on_liftoff("Liftoff Printer", |_| Box::pin(async move {
///         println!("Stalling liftoff for a second...");
///         rocket::tokio::time::sleep(std::time::Duration::from_secs(1)).await;
///         println!("And we're off!");
///     })));
///
/// rocket::execute(rocket.launch());
/// ```
///
/// Launch a pre-built instance of Rocket and wait for it to shutdown:
///
/// ```rust,no_run
/// use rocket::{Rocket, Ignite, Phase, Error};
///
/// fn launch<P: Phase>(rocket: Rocket<P>) -> Result<Rocket<Ignite>, Error> {
///     rocket::execute(rocket.launch())
/// }
/// ```
///
/// Do async work to build an instance of Rocket, launch, and wait for shutdown:
///
/// ```rust,no_run
/// use rocket::fairing::AdHoc;
///
/// // This line can also be inside of the `async` block.
/// let rocket = rocket::build();
///
/// rocket::execute(async move {
///     let rocket = rocket.ignite().await?;
///     let config = rocket.config();
///     rocket.launch().await
/// });
/// ```
pub fn execute<R, F>(future: F) -> R
    where F: std::future::Future<Output = R> + Send
{
    async_main(future)
}
