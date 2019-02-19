#![feature(specialization)]
#![feature(proc_macro_hygiene)]
#![feature(try_from)]
#![feature(crate_visibility_modifier)]
#![feature(doc_cfg)]
#![recursion_limit = "512"]

//! Types that map to concepts in HTTP.
//!
//! This module exports types that map to HTTP concepts or to the underlying
//! HTTP library when needed. Because the underlying HTTP library is likely to
//! change (see [#17]), types in [`hyper`] should be considered unstable.
//!
//! [#17]: https://github.com/SergioBenitez/Rocket/issues/17

#[macro_use]
extern crate pear;
extern crate cookie;
extern crate indexmap;
extern crate percent_encoding;
extern crate smallvec;
extern crate state;
extern crate time;
extern crate unicode_xid;

pub mod ext;
pub mod hyper;
pub mod uri;

#[doc(hidden)]
#[cfg(feature = "tls")]
pub mod tls;

#[doc(hidden)]
pub mod route;

#[macro_use]
mod docify;
#[macro_use]
mod known_media_types;
mod accept;
mod content_type;
mod cookies;
mod header;
mod media_type;
mod method;
mod raw_str;
mod status;

crate mod parse;

pub mod uncased;

#[doc(hidden)]
pub mod private {
    // We need to export these for codegen, but otherwise it's unnecessary.
    // TODO: Expose a `const fn` from ContentType when possible. (see RFC#1817)
    // FIXME(rustc): These show up in the rexported module.
    pub use media_type::{MediaParams, Source};
    pub use parse::Indexed;
    pub use smallvec::{Array, SmallVec};

    // This one we need to expose for core.
    pub use cookies::{CookieJar, Key};
}

pub use accept::{Accept, QMediaType};
pub use content_type::ContentType;
pub use header::{Header, HeaderMap};
pub use method::Method;
pub use raw_str::RawStr;
pub use status::{Status, StatusClass};

pub use cookies::{Cookie, Cookies, SameSite};
pub use media_type::MediaType;
