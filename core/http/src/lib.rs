#![feature(specialization)]
#![feature(proc_macro_hygiene)]
#![feature(crate_visibility_modifier)]
#![feature(doc_cfg)]
#![recursion_limit="512"]

#![warn(rust_2018_idioms)]

//! Types that map to concepts in HTTP.
//!
//! This module exports types that map to HTTP concepts or to the underlying
//! HTTP library when needed. Because the underlying HTTP library is likely to
//! change (see [#17]), types in [`hyper`] should be considered unstable.
//!
//! [#17]: https://github.com/SergioBenitez/Rocket/issues/17

#[macro_use] extern crate pear;

pub mod hyper;
pub mod uri;
pub mod ext;

#[doc(hidden)]
#[cfg(feature = "tls")]
pub mod tls;

#[doc(hidden)]
pub mod route;

#[macro_use]
mod docify;
#[macro_use]
mod known_media_types;
mod cookies;
mod method;
mod media_type;
mod content_type;
mod status;
mod header;
mod accept;
mod raw_str;

crate mod parse;

pub mod uncased;

#[doc(hidden)]
pub mod private {
    // We need to export these for codegen, but otherwise it's unnecessary.
    // TODO: Expose a `const fn` from ContentType when possible. (see RFC#1817)
    // FIXME(rustc): These show up in the rexported module.
    pub use crate::parse::Indexed;
    pub use crate::media_type::{MediaParams, Source};
    pub use smallvec::{SmallVec, Array};

    // This one we need to expose for core.
    pub use crate::cookies::{Key, CookieJar};
}

pub use crate::method::Method;
pub use crate::content_type::ContentType;
pub use crate::accept::{Accept, QMediaType};
pub use crate::status::{Status, StatusClass};
pub use crate::header::{Header, HeaderMap};
pub use crate::raw_str::RawStr;

pub use crate::media_type::MediaType;
pub use crate::cookies::{Cookie, SameSite, Cookies};
