//! Types that map to concepts in HTTP.
//!
//! This module exports types that map to HTTP concepts or to the underlying
//! HTTP library when needed.

pub use hyper::Version;

mod cookies;

#[doc(inline)]
pub use rocket_http::*;

#[doc(inline)]
pub use cookies::*;
