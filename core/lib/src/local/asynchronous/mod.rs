//! Structures for asynchronous local dispatching of requests, primarily for
//! testing.
//!
//! This module contains the `asynchronous` variant of the `local` API: it can
//! be used with `#[rocket::async_test]` or another asynchronous test harness.

mod client;
mod request;
mod response;

pub use client::*;
pub use request::*;
pub use response::*;
