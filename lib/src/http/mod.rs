//! Types that map to concepts in HTTP.
//!
//! This module exports types that map to HTTP concepts or to the underlying
//! HTTP library when needed. Because the underlying HTTP library is likely to
//! change (see <a
//! href="https://github.com/SergioBenitez/Rocket/issues/17">#17</a>), types in
//! [hyper](hyper/index.html) should be considered unstable.
pub mod hyper;
pub mod uri;

mod cookies;
mod method;
mod content_type;
mod status;
mod header;

// We need to export this for codegen, but otherwise it's unnecessary.
// TODO: Expose a `const fn` from ContentType when possible. (see RFC#1817)
#[doc(hidden)] pub mod ascii;

pub use self::method::Method;
pub use self::content_type::ContentType;
pub use self::status::{Status, StatusClass};
pub use self::header::{Header, HeaderMap};

pub use self::cookies::{Cookie, Cookies};
