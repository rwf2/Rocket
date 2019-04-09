#![feature(specialization)]
#![feature(proc_macro_hygiene)]
#![feature(crate_visibility_modifier)]
#![feature(doc_cfg)]
#![recursion_limit="512"]

//! Types that map to concepts in HTTP.
//!
//! This module exports types that map to HTTP concepts or to the underlying
//! HTTP library when needed. Because the underlying HTTP library is likely to
//! change (see [#17]), types in [`hyper`] should be considered unstable.
//!
//! [#17]: https://github.com/SergioBenitez/Rocket/issues/17

#[macro_use] extern crate pear;
extern crate percent_encoding;
extern crate smallvec;
extern crate cookie;
extern crate time;
extern crate indexmap;
extern crate state;
extern crate unicode_xid;

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
    pub use parse::Indexed;
    pub use media_type::{MediaParams, Source};
    pub use smallvec::{SmallVec, Array};

    // This one we need to expose for core.
    pub use cookies::{Key, CookieJar};
}

pub use method::Method;
pub use content_type::ContentType;
pub use accept::{Accept, QMediaType};
pub use status::{Status, StatusClass};
pub use header::{Header, HeaderMap};
pub use raw_str::RawStr;

pub use media_type::MediaType;
pub use cookies::{Cookie, SameSite, Cookies};
