//! Types that map to concepts in HTTP.
//!
//! This module exports types that map to HTTP concepts or to the underlying
//! HTTP library when needed. Because the underlying HTTP library is likely to
//! change (see <a
//! href="https://github.com/SergioBenitez/Rocket/issues/17">#17</a>), types in
//! [hyper](hyper/index.html) should be considered unstable.
pub mod hyper;
pub mod uri;

#[macro_use]
mod known_media_types;
mod cookies;
mod session;
mod method;
mod media_type;
mod content_type;
mod status;
mod header;
mod accept;
mod raw_str;

pub(crate) mod parse;

// We need to export these for codegen, but otherwise it's unnecessary.
// TODO: Expose a `const fn` from ContentType when possible. (see RFC#1817)
pub mod uncased;
#[doc(hidden)] pub use self::parse::IndexedStr;
#[doc(hidden)] pub use self::media_type::MediaParams;

pub use self::method::Method;
pub use self::content_type::ContentType;
pub use self::accept::{Accept, WeightedMediaType};
pub use self::status::{Status, StatusClass};
pub use self::header::{Header, HeaderMap};
pub use self::raw_str::RawStr;

pub use self::media_type::MediaType;
pub use self::cookies::*;
pub use self::session::*;

use smallvec::{Array, SmallVec};

pub trait IntoCollection<T> {
    fn into_collection<A: Array<Item=T>>(self) -> SmallVec<A>;
    fn mapped<U, F: FnMut(T) -> U, A: Array<Item=U>>(self, f: F) -> SmallVec<A>;
}

impl<T> IntoCollection<T> for T {
    #[inline]
    fn into_collection<A: Array<Item=T>>(self) -> SmallVec<A> {
        let mut vec = SmallVec::new();
        vec.push(self);
        vec
    }

    #[inline(always)]
    fn mapped<U, F: FnMut(T) -> U, A: Array<Item=U>>(self, mut f: F) -> SmallVec<A> {
        f(self).into_collection()
    }
}

impl<T> IntoCollection<T> for Vec<T> {
    #[inline(always)]
    fn into_collection<A: Array<Item=T>>(self) -> SmallVec<A> {
        SmallVec::from_vec(self)
    }

    #[inline]
    fn mapped<U, F: FnMut(T) -> U, A: Array<Item=U>>(self, mut f: F) -> SmallVec<A> {
        self.into_iter().map(|item| f(item)).collect()
    }
}

impl<'a, T: Clone> IntoCollection<T> for &'a [T] {
    #[inline(always)]
    fn into_collection<A: Array<Item=T>>(self) -> SmallVec<A> {
        self.iter().cloned().collect()
    }

    #[inline]
    fn mapped<U, F: FnMut(T) -> U, A: Array<Item=U>>(self, mut f: F) -> SmallVec<A> {
        self.iter().cloned().map(|item| f(item)).collect()
    }
}
