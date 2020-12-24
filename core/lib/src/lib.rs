#![recursion_limit="256"]

#![doc(html_root_url = "https://api.rocket.rs/master")]
#![doc(html_favicon_url = "https://rocket.rs/images/favicon.ico")]
#![doc(html_logo_url = "https://rocket.rs/images/logo-boxed.png")]
#![cfg_attr(nightly, feature(doc_cfg))]

#![warn(rust_2018_idioms)]

//! # Rocket - Core API Documentation
//!
//! Hello, and welcome to the core Rocket API documentation!
//!
//! This API documentation is highly technical and is purely a reference.
//! There's an [overview] of Rocket on the main site as well as a [full,
//! detailed guide]. If you'd like pointers on getting started, see the
//! [quickstart] or [getting started] chapters of the guide.
//!
//! You may also be interested in looking at the
//! [`rocket_contrib`](../rocket_contrib) documentation, which contains
//! automatic JSON (de)serialiazation, templating support, static file serving,
//! and other useful features.
//!
//! [overview]: https://rocket.rs/master/overview
//! [full, detailed guide]: https://rocket.rs/master/guide
//! [quickstart]: https://rocket.rs/master/guide/quickstart
//! [getting started]: https://rocket.rs/master/guide/getting-started
//!
//! ## Libraries
//!
//! Rocket's functionality is split into two crates:
//!
//!   1. Core - This core library. Needed by every Rocket application.
//!   2. [Contrib](../rocket_contrib) - Provides useful functionality for many
//!      Rocket applications. Completely optional.
//!
//! ## Usage
//!
//! Depend on `rocket` in `Rocket.toml`:
//!
//! ```toml
//! [dependencies]
//! rocket = "0.5.0-dev"
//! ```
//!
//! See the [guide](https://rocket.rs/master/guide) for more information on how to
//! write Rocket applications. Here's a simple example to get you started:
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
//!     rocket::ignite().mount("/", routes![hello])
//! }
//! ```
//!
//! ## Features
//!
//! There are two optional, disabled-by-default features:
//!
//!   * **secrets:** Enables support for [private cookies].
//!   * **tls:** Enables support for [TLS].
//!
//! The features can be enabled in `Rocket.toml`:
//!
//! ```toml
//! [dependencies]
//! rocket = { version = "0.5.0-dev", features = ["secrets", "tls"] }
//! ```
//!
//! [private cookies]: https://rocket.rs/master/guide/requests/#private-cookies
//! [TLS]: https://rocket.rs/master/guide/configuration/#tls
//!
//! ## Configuration
//!
//! Rocket and Rocket libraries are configured via the `Rocket.toml` file and/or
//! `ROCKET_{PARAM}` environment variables. For more information on how to
//! configure Rocket, see the [configuration section] of the guide as well as
//! the [`config`] module documentation.
//!
//! [configuration section]: https://rocket.rs/master/guide/configuration/
//!
//! ## Testing
//!
//! The [`local`] module contains structures that facilitate unit and
//! integration testing of a Rocket application. The top-level [`local`] module
//! documentation and the [testing chapter of the guide] include detailed
//! examples.
//!
//! [testing chapter of the guide]: https://rocket.rs/master/guide/testing/#testing

#[allow(unused_imports)] #[macro_use] extern crate rocket_codegen;
pub use rocket_codegen::*;
pub use async_trait::*;

#[macro_use] extern crate log;

/// These are public dependencies! Update docs if these are changed, especially
/// figment's version number in docs.
#[doc(hidden)]
pub use yansi;
pub use futures;
pub use tokio;
pub use figment;

#[doc(hidden)] #[macro_use] pub mod logger;
#[macro_use] pub mod outcome;
pub mod local;
pub mod request;
pub mod response;
pub mod config;
pub mod data;
pub mod handler;
pub mod fairing;
pub mod error;
pub mod catcher;

// Reexport of HTTP everything.
pub mod http {
    //! Types that map to concepts in HTTP.
    //!
    //! This module exports types that map to HTTP concepts or to the underlying
    //! HTTP library when needed.

    #[doc(inline)]
    pub use rocket_http::*;
}

mod shutdown;
mod router;
mod rocket;
mod server;
mod codegen;
mod ext;

#[doc(hidden)] pub use log::{info, warn, error, debug};
#[doc(inline)] pub use crate::response::Response;
#[doc(hidden)] pub use crate::codegen::{StaticRouteInfo, StaticCatcherInfo};
#[doc(inline)] pub use crate::data::Data;
#[doc(inline)] pub use crate::config::Config;
#[doc(inline)] pub use crate::catcher::Catcher;
pub use crate::router::Route;
pub use crate::request::{Request, State};
pub use crate::rocket::Rocket;
pub use crate::shutdown::Shutdown;

/// Alias to [`Rocket::ignite()`] Creates a new instance of `Rocket`.
pub fn ignite() -> Rocket {
    Rocket::ignite()
}

/// Alias to [`Rocket::custom()`]. Creates a new instance of `Rocket` with a
/// custom configuration provider.
pub fn custom<T: figment::Provider>(provider: T) -> Rocket {
    Rocket::custom(provider)
}

/// WARNING: This is unstable! Do not use this method outside of Rocket!
#[doc(hidden)]
pub fn async_test<R>(fut: impl std::future::Future<Output = R> + Send) -> R {
    tokio::runtime::Builder::new_multi_thread()
        .thread_name("rocket-test-worker-thread")
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("create tokio runtime")
        .block_on(fut)
}

/// WARNING: This is unstable! Do not use this method outside of Rocket!
#[doc(hidden)]
pub fn async_main<R>(fut: impl std::future::Future<Output = R> + Send) -> R {
    // FIXME: The `workers` value won't reflect swaps of `Rocket` in attach
    // fairings with different config values, or values from non-Rocket configs.
    // See tokio-rs/tokio#3329 for a necessary solution in `tokio`.
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(Config::from(Config::figment()).workers)
        .thread_name("rocket-worker-thread")
        .enable_all()
        .build()
        .expect("create tokio runtime")
        .block_on(fut)
}
