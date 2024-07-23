//! Server and application configuration.
//!
//! See the [configuration guide] for full details.
//!
//! [configuration guide]: https://rocket.rs/master/guide/configuration/
//!
//! ## Extracting Configuration Parameters
//!
//! Rocket exposes the active [`Figment`] via [`Rocket::figment()`]. Any value
//! that implements [`Deserialize`] can be extracted from the figment:
//!
//! ```rust
//! use rocket::fairing::AdHoc;
//!
//! #[derive(serde::Deserialize)]
//! struct AppConfig {
//!     id: Option<usize>,
//!     port: u16,
//! }
//!
//! #[rocket::launch]
//! fn rocket() -> _ {
//!     rocket::build().attach(AdHoc::config::<AppConfig>())
//! }
//! ```
//!
//! [`Figment`]: figment::Figment
//! [`Rocket::figment()`]: crate::Rocket::figment()
//! [`Rocket::figment()`]: crate::Rocket::figment()
//! [`Deserialize`]: serde::Deserialize
//!
//! ## Workers
//!
//! The `workers` parameter sets the number of threads used for parallel task
//! execution; there is no limit to the number of concurrent tasks. Due to a
//! limitation in upstream async executers, unlike other values, the `workers`
//! configuration value cannot be reconfigured or be configured from sources
//! other than those provided by [`Config::figment()`]. In other words, only the
//! values set by the `ROCKET_WORKERS` environment variable or in the `workers`
//! property of `Rocket.toml` will be considered - all other `workers` values
//! are ignored.
//!
//! ## Custom Providers
//!
//! A custom provider can be set via [`rocket::custom()`], which replaces calls to
//! [`rocket::build()`]. The configured provider can be built on top of
//! [`Config::figment()`], [`Config::default()`], both, or neither. The
//! [Figment](figment) documentation has full details on instantiating existing
//! providers like [`Toml`]() and [`Env`] as well as creating custom providers for
//! more complex cases.
//!
//! Configuration values can be overridden at runtime by merging figment's tuple
//! providers with Rocket's default provider:
//!
//! ```rust
//! # #[macro_use] extern crate rocket;
//! use rocket::data::{Limits, ToByteUnit};
//!
//! #[launch]
//! fn rocket() -> _ {
//!     let figment = rocket::Config::figment()
//!         .merge(("port", 1111))
//!         .merge(("limits", Limits::new().limit("json", 2.mebibytes())));
//!
//!     rocket::custom(figment).mount("/", routes![/* .. */])
//! }
//! ```
//!
//! An application that wants to use Rocket's defaults for [`Config`], but not
//! its configuration sources, while allowing the application to be configured
//! via an `App.toml` file that uses top-level keys as profiles (`.nested()`)
//! and `APP_` environment variables as global overrides (`.global()`), and
//! `APP_PROFILE` to configure the selected profile, can be structured as
//! follows:
//!
//! ```rust
//! # #[macro_use] extern crate rocket;
//! use serde::{Serialize, Deserialize};
//! use figment::{Figment, Profile, providers::{Format, Toml, Serialized, Env}};
//! use rocket::fairing::AdHoc;
//!
//! #[derive(Debug, Deserialize, Serialize)]
//! struct Config {
//!     app_value: usize,
//!     /* and so on.. */
//! }
//!
//! impl Default for Config {
//!     fn default() -> Config {
//!         Config { app_value: 3, }
//!     }
//! }
//!
//! #[launch]
//! fn rocket() -> _ {
//!     let figment = Figment::from(rocket::Config::default())
//!         .merge(Serialized::defaults(Config::default()))
//!         .merge(Toml::file("App.toml").nested())
//!         .merge(Env::prefixed("APP_").global())
//!         .select(Profile::from_env_or("APP_PROFILE", "default"));
//!
//!     rocket::custom(figment)
//!         .mount("/", routes![/* .. */])
//!         .attach(AdHoc::config::<Config>())
//! }
//! ```
//!
//! [`rocket::custom()`]: crate::custom()
//! [`rocket::build()`]: crate::build()
//! [`Toml`]: figment::providers::Toml
//! [`Env`]: figment::providers::Env

#[macro_use]
mod ident;
mod config;
mod cli_colors;
mod http_header;
#[cfg(test)]
mod tests;

pub use ident::Ident;
pub use config::Config;
pub use cli_colors::CliColors;

pub use crate::trace::{TraceFormat, Level};
pub use crate::shutdown::ShutdownConfig;

#[cfg(feature = "tls")]
pub use crate::tls::TlsConfig;

#[cfg(feature = "mtls")]
pub use crate::mtls::MtlsConfig;

#[cfg(feature = "secrets")]
mod secret_key;

#[cfg(unix)]
pub use crate::shutdown::Sig;

#[cfg(feature = "secrets")]
pub use secret_key::{SecretKey, Cipher};
