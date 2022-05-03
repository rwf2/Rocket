#![recursion_limit="512"]

#![warn(rust_2018_idioms)]
#![warn(missing_docs)]

//! Types that map to concepts in HTTP.
//!
//! This module exports types that map to HTTP concepts or to the underlying
//! HTTP library when needed. Because the underlying HTTP library is likely to
//! change (see [#17]), types in [`hyper`] should be considered unstable.
//!
//! [#17]: https://github.com/SergioBenitez/Rocket/issues/17

#[macro_use]
extern crate pear;

pub mod hyper;
pub mod uri;
pub mod ext;

#[macro_use]
mod docify;

#[macro_use]
mod header;
mod method;
mod status;
mod raw_str;
mod parse;
mod listener;

/// Case-preserving, ASCII case-insensitive string types.
///
/// An _uncased_ string is case-preserving. That is, the string itself contains
/// cased characters, but comparison (including ordering, equality, and hashing)
/// is ASCII case-insensitive. **Note:** the `alloc` feature _is_ enabled.
pub mod uncased {
    #[doc(inline)] pub use uncased::*;
}

// Types that we expose for use _only_ by core. Please don't use this.
#[doc(hidden)]
#[path = "."]
pub mod private {
    pub use crate::parse::Indexed;
    pub use smallvec::{SmallVec, Array};
    pub use crate::listener::{TcpListener, Incoming, Listener, Connection, Certificates};
    pub use cookie;
}

#[doc(hidden)]
#[cfg(feature = "tls")]
pub mod tls;

pub use crate::method::Method;
pub use crate::status::{Status, StatusClass};
pub use crate::raw_str::{RawStr, RawStrBuf};
pub use crate::header::*;
