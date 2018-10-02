#![feature(crate_visibility_modifier)]
#![feature(never_type)]
#![feature(doc_cfg)]
#![doc(html_root_url = "https://api.rocket.rs/v0.4")]
#![doc(html_favicon_url = "https://rocket.rs/v0.4/images/favicon.ico")]
#![doc(html_logo_url = "https://rocket.rs/v0.4/images/logo-boxed.png")]

//! This crate contains officially sanctioned contributor libraries that provide
//! functionality commonly used by Rocket applications.
//!
//! These libraries are always kept in-sync with the core Rocket library. They
//! provide common, but not fundamental, abstractions to be used by Rocket
//! applications.
//!
//! Each module in this library is held behind a feature flag, with the most
//! common modules exposed by default. The present feature list is below, with
//! an asterisk next to the features that are enabled by default:
//!
//! * [json*](type@json) - JSON (de)serialization
//! * [serve*](serve) - Static File Serving
//! * [msgpack](msgpack) - MessagePack (de)serialization
//! * [handlebars_templates](templates) - Handlebars Templating
//! * [tera_templates](templates) - Tera Templating
//! * [uuid](uuid) - UUID (de)serialization
//! * [${database}_pool](databases) - Database Configuration and Pooling
//!
//! The recommend way to include features from this crate via Cargo in your
//! project is by adding a `[dependencies.rocket_contrib]` section to your
//! `Cargo.toml` file, setting `default-features` to false, and specifying
//! features manually. For example, to use the JSON module, you would add:
//!
//! ```toml
//! [dependencies.rocket_contrib]
//! version = "0.4.0-rc.1"
//! default-features = false
//! features = ["json"]
//! ```
//!
//! This crate is expected to grow with time, bringing in outside crates to be
//! officially supported by Rocket.

#[allow(unused_imports)]
#[macro_use]
extern crate log;
#[allow(unused_imports)]
#[macro_use]
extern crate rocket;

#[cfg(feature = "json")]
#[macro_use]
pub mod json;
#[cfg(feature = "databases")]
pub mod databases;
#[cfg(feature = "msgpack")]
pub mod msgpack;
#[cfg(feature = "serve")]
pub mod serve;
#[cfg(feature = "templates")]
pub mod templates;
#[cfg(feature = "uuid")]
pub mod uuid;

#[cfg(feature = "databases")]
extern crate rocket_contrib_codegen;
#[cfg(feature = "databases")]
#[doc(hidden)]
pub use rocket_contrib_codegen::*;
#[cfg(feature = "json")]
extern crate serde_json;

#[cfg(feature = "json")]
pub use serde_json::{json_internal, json_internal_vec};

#[cfg(feature = "handlebars_templates")]
pub extern crate handlebars;

#[cfg(feature = "tera_templates")]
pub extern crate tera;

#[cfg(feature = "brotli_compression")]
extern crate brotli;
#[cfg(feature = "gzip_compression")]
extern crate flate2;

#[cfg(feature = "json")]
#[cfg_attr(feature = "json", macro_use)]
#[doc(hidden)]
pub mod json;

#[cfg(feature = "json")]
pub use json::{Json, JsonError, JsonValue};

#[cfg(feature = "msgpack")]
#[doc(hidden)]
pub mod msgpack;

#[cfg(feature = "msgpack")]
pub use msgpack::{MsgPack, MsgPackError};

#[cfg(feature = "templates")]
mod templates;

#[cfg(feature = "templates")]
pub use templates::{Engines, Template, TemplateMetadata};

#[cfg(feature = "uuid")]
mod uuid;

#[cfg(feature = "uuid")]
pub use uuid::{Uuid, UuidParseError};

#[cfg(feature = "static_files")]
pub mod static_files;

#[cfg(feature = "database_pool")]
pub mod databases;

#[cfg(feature = "database_pool_codegen")]
#[allow(unused_imports)]
#[macro_use]
extern crate rocket_contrib_codegen;

#[cfg(feature = "database_pool_codegen")]
#[doc(hidden)]
pub use rocket_contrib_codegen::*;

#[cfg(any(feature = "brotli_compression", feature = "gzip_compression"))]
mod compression;

#[cfg(any(feature = "brotli_compression", feature = "gzip_compression"))]
pub use compression::Compression;
