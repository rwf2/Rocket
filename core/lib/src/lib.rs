#![feature(specialization)]
#![feature(decl_macro)]
#![feature(try_trait)]
#![feature(fnbox)]
#![feature(never_type)]
#![feature(proc_macro_hygiene)]
#![feature(crate_visibility_modifier)]
#![feature(try_from)]
#![feature(label_break_value)]

#![recursion_limit="256"]

#![doc(html_root_url = "https://api.rocket.rs/v0.4")]
#![doc(html_favicon_url = "https://rocket.rs/v0.4/images/favicon.ico")]
#![doc(html_logo_url = "https://rocket.rs/v0.4/images/logo-boxed.png")]

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
//! [overview]: https://rocket.rs/v0.4/overview
//! [full, detailed guide]: https://rocket.rs/v0.4/guide
//! [quickstart]: https://rocket.rs/v0.4/guide/quickstart
//! [getting started]: https://rocket.rs/v0.4/guide/getting-started
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
//! First, depend on `rocket` in `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! rocket = "0.4.0"
//! ```
//!
//! Then, add the following to the top of your `main.rs` file:
//!
//! ```rust
//! #![feature(proc_macro_hygiene, decl_macro)]
//!
//! #[macro_use] extern crate rocket;
//! # #[get("/")] fn hello() { }
//! # fn main() { rocket::ignite().mount("/", routes![hello]); }
//! ```
//!
//! See the [guide](https://rocket.rs/v0.4/guide) for more information on how to
//! write Rocket applications. Here's a simple example to get you started:
//!
//! ```rust
//! #![feature(proc_macro_hygiene, decl_macro)]
//!
//! #[macro_use] extern crate rocket;
//!
//! #[get("/")]
//! fn hello() -> &'static str {
//!     "Hello, world!"
//! }
//!
//! fn main() {
//! # if false { // We don't actually want to launch the server in an example.
//!     rocket::ignite().mount("/", routes![hello]).launch();
//! # }
//! }
//! ```
//!
//! ## Configuration
//!
//! Rocket and Rocket libraries are configured via the `Rocket.toml` file and/or
//! `ROCKET_{PARAM}` environment variables. For more information on how to
//! configure Rocket, see the [configuration section] of the guide as well as
//! the [`config`] module documentation.
//!
//! [configuration section]: https://rocket.rs/v0.4/guide/configuration/
//!
//! ## Testing
//!
//! The [`local`] module contains structures that facilitate unit and
//! integration testing of a Rocket application. The top-level [`local`] module
//! documentation and the [testing chapter of the guide] include detailed
//! examples.
//!
//! [testing chapter of the guide]: https://rocket.rs/v0.4/guide/testing/#testing

#[allow(unused_imports)] #[macro_use] extern crate rocket_codegen;
#[doc(hidden)] pub use rocket_codegen::*;

extern crate rocket_http;
#[macro_use] extern crate log;
#[macro_use] extern crate pear;
extern crate yansi;
extern crate toml;
extern crate num_cpus;
extern crate state;
extern crate time;
extern crate memchr;
extern crate base64;
extern crate isatty;

#[cfg(test)] #[macro_use] extern crate lazy_static;

#[doc(hidden)] #[macro_use] pub mod logger;
pub mod local;
pub mod request;
pub mod response;
pub mod outcome;
pub mod config;
pub mod data;
pub mod handler;
pub mod fairing;
pub mod error;

// Reexport of HTTP everything.
pub mod http {
    //! Types that map to concepts in HTTP.
    //!
    //! This module exports types that map to HTTP concepts or to the underlying
    //! HTTP library when needed.

    #[doc(inline)]
    pub use rocket_http::*;
}

mod router;
mod rocket;
mod codegen;
mod catcher;
mod ext;

#[doc(inline)] pub use response::Response;
#[doc(inline)] pub use handler::{Handler, ErrorHandler};
#[doc(hidden)] pub use codegen::{StaticRouteInfo, StaticCatchInfo};
#[doc(inline)] pub use outcome::Outcome;
#[doc(inline)] pub use data::Data;
#[doc(inline)] pub use config::Config;
pub use router::Route;
pub use request::{Request, State};
pub use catcher::Catcher;
pub use rocket::Rocket;

/// Alias to [`Rocket::ignite()`] Creates a new instance of `Rocket`.
pub fn ignite() -> Rocket {
    Rocket::ignite()
}

/// Alias to [`Rocket::custom()`]. Creates a new instance of `Rocket` with a
/// custom configuration.
pub fn custom(config: config::Config) -> Rocket {
    Rocket::custom(config)
}
