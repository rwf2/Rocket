//! Asynchronous local dispatching of requests.
//!
//! This module contains the `asynchronous` variant of the `local` API: it can
//! be used with `#[rocket::async_test]` or another asynchronous test harness.
//! For the blocking variant, see [`blocking`](super::blocking).
//!
//! See the [top-level documentation](super) for more usage details.

mod client;
mod request;
mod response;

pub use client::*;
pub use request::*;
pub use response::*;
