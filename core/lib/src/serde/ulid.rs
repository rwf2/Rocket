//! [ULID](https://github.com/ulid/spec) path/query parameter and form value parsing support.
//!
//! # Enabling
//!
//! This module is only available when the `ulid` feature is enabled. Enable it
//! in `Cargo.toml` as follows:
//!
//! ```toml
//! [dependencies.rocket]
//! version = "0.5.0-rc.2"
//! features = ["ulid"]
//! ```
//!
//! # Usage
//!
//! `Ulid` implements [`FromParam`] and [`FromFormField`] (i.e,
//! [`FromForm`](crate::form::FromForm)), allowing ULID values to be accepted
//! directly in paths, queries, and forms. You can use the `Ulid` type directly
//! as a target of a dynamic parameter:
//!
//! ```rust
//! # #[macro_use] extern crate rocket;
//! use rocket::serde::ulid::Ulid;
//!
//! #[get("/users/<id>")]
//! fn user(id: Ulid) -> String {
//!     format!("We found: {}", id)
//! }
//! ```
//!
//! You can also use the `Ulid` as a form value, including in query strings:
//!
//! ```rust
//! # #[macro_use] extern crate rocket;
//! use rocket::serde::ulid::Ulid;
//!
//! #[get("/user?<id>")]
//! fn user(id: Ulid) -> String {
//!     format!("User ID: {}", id)
//! }
//! ```
//!
//! Additionally, `Ulid` implements `UriDisplay<P>` for all `P`. As such, route
//! URIs including `Ulid`s can be generated in a type-safe manner:
//!
//! ```rust
//! # #[macro_use] extern crate rocket;
//! use rocket::serde::ulid::Ulid;
//! use rocket::response::Redirect;
//!
//! #[get("/user/<id>")]
//! fn user(id: Ulid) -> String {
//!     format!("User ID: {}", id)
//! }
//!
//! #[get("/user?<id>")]
//! fn old_user_path(id: Ulid) -> Redirect {
//!     # let _ = Redirect::to(uri!(user(&id)));
//!     # let _ = Redirect::to(uri!(old_user_path(id)));
//!     # let _ = Redirect::to(uri!(old_user_path(&id)));
//!     Redirect::to(uri!(user(id)))
//! }
//! ```
//!

use crate::form::{self, FromFormField, ValueField};
use crate::request::FromParam;

/// Error returned on [`FromParam`] or [`FromFormField`] failure.
///
pub use ulid_::DecodingError;

pub use ulid_::Ulid;

impl<'a> FromParam<'a> for Ulid {
    type Error = DecodingError;

    /// A value is successfully parsed if `param` is a properly formatted Ulid.
    /// Otherwise, an error is returned.
    #[inline(always)]
    fn from_param(param: &'a str) -> Result<Ulid, Self::Error> {
        param.parse()
    }
}

impl<'v> FromFormField<'v> for Ulid {
    #[inline]
    fn from_value(field: ValueField<'v>) -> form::Result<'v, Self> {
        Ok(field.value.parse().map_err(form::error::Error::custom)?)
    }
}

#[cfg(test)]
mod test {
    use super::{FromParam, Ulid};

    #[test]
    fn test_from_param() {
        let ulid_str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
        let ulid = Ulid::from_param(ulid_str).unwrap();
        assert_eq!(ulid_str, ulid.to_string());
    }

    #[test]
    fn test_from_param_invalid() {
        let ulid_str = "01ARZ3NDEKTSV4RRFFQ69G5FAU";
        assert!(Ulid::from_param(ulid_str).is_err());
    }
}
