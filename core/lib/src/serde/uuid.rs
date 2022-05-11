//! UUID path/query parameter and form value parsing support.
//!
//! # Enabling
//!
//! This module is only available when the `uuid` feature is enabled. Enable it
//! in `Cargo.toml` as follows:
//!
//! ```toml
//! [dependencies.rocket]
//! version = "0.5.0-rc.2"
//! features = ["uuid"]
//! ```
//!
//! # Usage
//!
//! `Uuid` implements [`FromParam`] and [`FromFormField`] (i.e,
//! [`FromForm`](crate::form::FromForm)), allowing UUID values to be accepted
//! directly in paths, queries, and forms. You can use the `Uuid` type directly
//! as a target of a dynamic parameter:
//!
//! ```rust
//! # #[macro_use] extern crate rocket;
//! use rocket::serde::uuid::Uuid;
//!
//! #[get("/users/<id>")]
//! fn user(id: Uuid) -> String {
//!     format!("We found: {}", id)
//! }
//! ```
//!
//! You can also use the `Uuid` as a form value, including in query strings:
//!
//! ```rust
//! # #[macro_use] extern crate rocket;
//! use rocket::serde::uuid::Uuid;
//!
//! #[get("/user?<id>")]
//! fn user(id: Uuid) -> String {
//!     format!("User ID: {}", id)
//! }
//! ```
//!
//! Additionally, `Uuid` implements `UriDisplay<P>` for all `P`. As such, route
//! URIs including `Uuid`s can be generated in a type-safe manner:
//!
//! ```rust
//! # #[macro_use] extern crate rocket;
//! use rocket::serde::uuid::Uuid;
//! use rocket::response::Redirect;
//!
//! #[get("/user/<id>")]
//! fn user(id: Uuid) -> String {
//!     format!("User ID: {}", id)
//! }
//!
//! #[get("/user?<id>")]
//! fn old_user_path(id: Uuid) -> Redirect {
//!     # let _ = Redirect::to(uri!(user(&id)));
//!     # let _ = Redirect::to(uri!(old_user_path(id)));
//!     # let _ = Redirect::to(uri!(old_user_path(&id)));
//!     Redirect::to(uri!(user(id)))
//! }
//! ```
//!
//! # Extra Features
//!
//! The [`uuid`](https://docs.rs/uuid/1) crate exposes extra `v{n}` features
//! for generating UUIDs which are not enabled by Rocket. To enable these
//! features, depend on `uuid` directly. The extra functionality can be accessed
//! via both `rocket::serde::uuid::Uuid` or the direct `uuid::Uuid`; the types
//! are one and the same.
//!
//! ```toml
//! [dependencies.uuid]
//! version = "1"
//! features = ["v1", "v4"]
//! ```

use crate::request::FromParam;
use crate::form::{self, FromFormField, ValueField};

/// Error returned on [`FromParam`] or [`FromFormField`] failure.
///
pub use uuid_::Error;

pub use uuid_::{Uuid, Builder, Variant, Version, Bytes, uuid, fmt};

impl<'a> FromParam<'a> for Uuid {
    type Error = Error;

    /// A value is successfully parsed if `param` is a properly formatted Uuid.
    /// Otherwise, an error is returned.
    #[inline(always)]
    fn from_param(param: &'a str) -> Result<Uuid, Self::Error> {
        param.parse()
    }
}

impl<'v> FromFormField<'v> for Uuid {
    #[inline]
    fn from_value(field: ValueField<'v>) -> form::Result<'v, Self> {
        Ok(field.value.parse().map_err(form::error::Error::custom)?)
    }
}

#[cfg(test)]
mod test {
    use super::{Uuid, FromParam};

    #[test]
    fn test_from_param() {
        let uuid_str = "c1aa1e3b-9614-4895-9ebd-705255fa5bc2";
        let uuid = Uuid::from_param(uuid_str).unwrap();
        assert_eq!(uuid_str, uuid.to_string());
    }

    #[test]
    fn test_from_param_invalid() {
        let uuid_str = "c1aa1e3b-9614-4895-9ebd-705255fa5bc2p";
        assert!(Uuid::from_param(uuid_str).is_err());
    }
}
