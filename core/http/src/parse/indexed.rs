#![allow(dead_code)]

use std::borrow::Cow;
use std::ops::{Index, Range};
use std::fmt::{self, Debug};

use pear::input::Length;

use crate::ext::IntoOwned;

pub use pear::input::Extent;

pub type IndexedStr<'a> = Indexed<'a, str>;
pub type IndexedBytes<'a> = Indexed<'a, [u8]>;

pub trait AsPtr {
    fn as_ptr(&self) -> *const u8;
}

impl AsPtr for str {
    #[inline(always)]
    fn as_ptr(&self) -> *const u8 {
        str::as_ptr(self)
    }
}

impl AsPtr for [u8] {
    #[inline(always)]
    fn as_ptr(&self) -> *const u8 {
        <[u8]>::as_ptr(self)
    }
}

/// Either a concrete string or indices to the start and end of a string.
#[derive(PartialEq)]
pub enum Indexed<'a, T: ?Sized + ToOwned> {
    /// The start and end index of a string.
    Indexed(usize, usize),
    /// A conrete string.
    Concrete(Cow<'a, T>)
}

impl<A, T: ?Sized + ToOwned> From<Extent<A>> for Indexed<'_, T> {
    fn from(e: Extent<A>) -> Self {
        Indexed::Indexed(e.start, e.end)
    }
}

impl<'a, T: ?Sized + ToOwned + 'a> From<Cow<'a, T>> for Indexed<'a, T> {
    #[inline(always)]
    fn from(value: Cow<'a, T>) -> Indexed<'a, T> {
        Indexed::Concrete(value)
    }
}

impl<'a, T: ?Sized + ToOwned + 'a> Indexed<'a, T> {
    /// Panics if `self` is not an `Indexed`.
    #[inline(always)]
    pub fn indices(self) -> (usize, usize) {
        match self {
            Indexed::Indexed(a, b) => (a, b),
            _ => panic!("cannot convert indexed T to U unless indexed")
        }
    }

    /// Panics if `self` is not an `Indexed`.
    #[inline(always)]
    pub fn coerce<U: ?Sized + ToOwned>(self) -> Indexed<'a, U> {
        match self {
            Indexed::Indexed(a, b) => Indexed::Indexed(a, b),
            _ => panic!("cannot convert indexed T to U unless indexed")
        }
    }

    /// Panics if `self` is not an `Indexed`.
    #[inline(always)]
    pub fn coerce_lifetime<'b>(self) -> Indexed<'b, T> {
        match self {
            Indexed::Indexed(a, b) => Indexed::Indexed(a, b),
            _ => panic!("cannot coerce lifetime unless indexed")
        }
    }
}

impl<T: 'static + ?Sized + ToOwned> IntoOwned for Indexed<'_, T> {
    type Owned = Indexed<'static, T>;

    fn into_owned(self) -> Indexed<'static, T> {
        match self {
            Indexed::Indexed(a, b) => Indexed::Indexed(a, b),
            Indexed::Concrete(cow) => Indexed::Concrete(IntoOwned::into_owned(cow))
        }
    }
}

use std::ops::Add;

impl<'a, T: ?Sized + ToOwned + 'a> Add for Indexed<'a, T> {
    type Output = Indexed<'a, T>;

    #[inline]
    fn add(self, other: Indexed<'a, T>) -> Indexed<'a, T> {
        match self {
            Indexed::Indexed(a, b) => match other {
                Indexed::Indexed(c, d) if b == c && a < d => Indexed::Indexed(a, d),
                _ => panic!("+ requires indexed")
            }
            _ => panic!("+ requires indexed")
        }
    }
}

impl<'a, T: ?Sized + ToOwned + 'a> Indexed<'a, T>
    where T: Length + AsPtr + Index<Range<usize>, Output = T>
{
    /// Returns `None` if `needle` is not a substring of `haystack`. Otherwise
    /// returns an `Indexed` with the indices of `needle` in `haystack`.
    pub fn checked_from(needle: &T, haystack: &T) -> Option<Indexed<'a, T>> {
        let needle_start = needle.as_ptr() as usize;
        let haystack_start = haystack.as_ptr() as usize;
        if needle_start < haystack_start {
            return None;
        }

        let needle_end = needle_start + needle.len();
        let haystack_end = haystack_start + haystack.len();
        if needle_end > haystack_end {
            return None;
        }

        let start = needle_start - haystack_start;
        let end = start + needle.len();
        Some(Indexed::Indexed(start, end))
    }

    /// Like `checked_from` but without checking if `needle` is indeed a
    /// substring of `haystack`.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `needle` is indeed a substring of
    /// `haystack`.
    pub unsafe fn unchecked_from(needle: &T, haystack: &T) -> Indexed<'a, T> {
        let haystack_start = haystack.as_ptr() as usize;
        let needle_start = needle.as_ptr() as usize;

        let start = needle_start - haystack_start;
        let end = start + needle.len();
        Indexed::Indexed(start, end)
    }

    /// Whether this string is derived from indexes or not.
    #[inline]
    pub fn is_indexed(&self) -> bool {
        match *self {
            Indexed::Indexed(..) => true,
            Indexed::Concrete(..) => false,
        }
    }

    /// Whether this string is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Make `self` concrete by allocating if indexed.
    ///
    /// # Panics
    ///
    /// Panics if `self` is an indexed string and `source` is None.
    pub fn into_concrete(self, source: &Option<Cow<'_, T>>) -> Cow<'a, T> {
        if self.is_indexed() && source.is_none() {
            panic!("cannot concretize indexed str to str without base string!")
        }

        match self {
            Indexed::Indexed(i, j) => Cow::Owned(source.as_ref().unwrap()[i..j].to_owned()),
            Indexed::Concrete(string) => string,
        }
    }

    /// Retrieves the string `self` corresponds to. If `self` is derived from
    /// indexes, the corresponding subslice of `source` is returned. Otherwise,
    /// the concrete string is returned.
    ///
    /// # Panics
    ///
    /// Panics if `self` is an indexed string and `source` is None.
    pub fn from_cow_source<'s>(&'s self, source: &'s Option<Cow<'_, T>>) -> &'s T {
        if self.is_indexed() && source.is_none() {
            panic!("cannot convert indexed str to str without base string!")
        }

        match *self {
            Indexed::Indexed(i, j) => &source.as_ref().unwrap()[i..j],
            Indexed::Concrete(ref mstr) => mstr.as_ref(),
        }
    }

    /// Retrieves the string `self` corresponds to. If `self` is derived from
    /// indexes, the corresponding subslice of `string` is returned. Otherwise,
    /// the concrete string is returned.
    ///
    /// # Panics
    ///
    /// Panics if `self` is an indexed string and `string` is None.
    pub fn from_source<'s>(&'s self, source: Option<&'s T>) -> &'s T {
        if self.is_indexed() && source.is_none() {
            panic!("Cannot convert indexed str to str without base string!")
        }

        match *self {
            Indexed::Indexed(i, j) => &source.unwrap()[(i as usize)..(j as usize)],
            Indexed::Concrete(ref mstr) => &*mstr,
        }
    }
}

impl<'a, T: ToOwned + ?Sized + 'a> Clone for Indexed<'a, T> {
    fn clone(&self) -> Self {
        match *self {
            Indexed::Indexed(a, b) => Indexed::Indexed(a, b),
            Indexed::Concrete(ref cow) => Indexed::Concrete(cow.clone())
        }
    }
}

impl<'a, T: ?Sized + 'a> Debug for Indexed<'a, T>
    where T: ToOwned + Debug, T::Owned: Debug
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Indexed::Indexed(a, b) => fmt::Debug::fmt(&(a, b), f),
            Indexed::Concrete(ref cow) => fmt::Debug::fmt(cow, f),
        }
    }
}

impl<'a, T: ?Sized + Length + ToOwned + 'a> Length for Indexed<'a, T> {
    #[inline(always)]
    fn len(&self) -> usize {
        match *self {
            Indexed::Indexed(a, b) => (b - a) as usize,
            Indexed::Concrete(ref cow) => cow.len()
        }
    }
}
