//! Structures for blocking local dispatching of requests, primarily for
//! testing.
//!
//! This module contains the `blocking` variant of the `local` API: it can be
//! used in Rust's synchronous `#[test]` harness. This is accomplished by
//! starting and running an interal asynchronous Runtime as needed.

mod client;
mod request;
mod response;

pub use self::client::*;
pub use self::request::*;
pub use self::response::*;
