#![feature(drop_types_in_const, macro_reexport)]
#![cfg_attr(feature = "templates", feature(conservative_impl_trait))]

// TODO: Version URLs.
#![doc(html_root_url = "https://api.rocket.rs/rocket_contrib/")]

//! This crate contains officially sanctioned contributor libraries that provide
//! functionality commonly used by Rocket applications.
//!
//! These libraries are always kept in-sync with the core Rocket library. They
//! provide common, but not fundamental, abstractions to be used by Rocket
//! applications. In particular, contributor libraries typically export types
//! implementing a combination of the `FromRequest`, `FromParam`, and
//! `Responder` traits.
//!
//! Each module in this library is held behind a feature flag, with the most
//! common modules exposed by default. The present feature list is below, with
//! an asterisk next to the features that are enabled by default:
//!
//! * [json*](struct.Json.html)
//! * [msgpack](struct.MsgPack.html)
//! * [handlebars_templates](struct.Template.html)
//! * [tera_templates](struct.Template.html)
//! * [uuid](struct.UUID.html)
//!
//! The recommend way to include features from this crate via Cargo in your
//! project is by adding a `[dependencies.rocket_contrib]` section to your
//! `Cargo.toml` file, setting `default-features` to false, and specifying
//! features manually. For example, to use the JSON module, you would add:
//!
//! ```toml,ignore
//! [dependencies.rocket_contrib]
//! version = "*"
//! default-features = false
//! features = ["json"]
//! ```
//!
//! This crate is expected to grow with time, bringing in outside crates to be
//! officially supported by Rocket.

#[macro_use] extern crate log;
#[macro_use] extern crate rocket;

#[cfg(feature = "serde")]
extern crate serde;

#[cfg(feature = "json")]
#[cfg_attr(feature = "json", macro_reexport(json_internal))]
extern crate serde_json;

#[cfg(feature = "json")]
#[cfg_attr(feature = "json", macro_use)]
#[doc(hidden)]
pub mod json;

#[cfg(feature = "json")]
pub use json::{Json, SerdeError, Value};

#[cfg(feature = "msgpack")]
#[doc(hidden)]
pub mod msgpack;

#[cfg(feature = "msgpack")]
pub use msgpack::{MsgPack, MsgPackError};

#[cfg(feature = "templates")]
mod templates;

#[cfg(feature = "templates")]
pub use templates::Template;

#[cfg(feature = "uuid")]
mod uuid;

#[cfg(feature = "uuid")]
pub use uuid::{UUID, UuidParseError};
