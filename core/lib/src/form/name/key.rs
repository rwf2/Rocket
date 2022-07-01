use std::ops::Deref;

use ref_cast::RefCast;

use crate::http::RawStr;

/// A field name key composed of indices.
///
/// A form field name key is composed of _indices_, delimited by `:`. The
/// graphic below illustrates this composition for a single field in
/// `$name=$value` format:
///
/// ```text
///       food.bart[bar:foo:baz]=some-value
/// name  |--------------------|
/// key   |--| |--| |---------|
/// index |--| |--| |-| |-| |-|
/// ```
///
/// A `Key` is a wrapper around a given key string with methods to easily access
/// its indices.
///
/// # Serialization
///
/// A value of this type is serialized exactly as an `&str` consisting of the
/// entire key.
#[repr(transparent)]
#[derive(RefCast, Debug, PartialEq, Eq, Hash)]
pub struct Key(str);

impl Key {
    /// Wraps a string as a `Key`. This is cost-free.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::form::name::Key;
    ///
    /// let key = Key::new("a:b:c");
    /// assert_eq!(key.as_str(), "a:b:c");
    /// ```
    pub fn new<S: AsRef<str> + ?Sized>(string: &S) -> &Key {
        Key::ref_cast(string.as_ref())
    }

    /// Returns an iterator over the indices of `self`, including empty indices.
    ///
    /// See the [top-level docs](Self) for a description of "indices".
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::form::name::Key;
    ///
    /// let key = Key::new("foo:bar::baz:a.b.c");
    /// let indices: Vec<_> = key.indices().collect();
    /// assert_eq!(indices, &["foo", "bar", "", "baz", "a.b.c"]);
    /// ```
    pub fn indices(&self) -> impl Iterator<Item = &str> {
        self.split(':')
    }

    /// Borrows the underlying string.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::form::name::Key;
    ///
    /// let key = Key::new("a:b:c");
    /// assert_eq!(key.as_str(), "a:b:c");
    /// ```
    pub fn as_str(&self) -> &str {
        &*self
    }
}

impl Deref for Key {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl serde::Serialize for Key {
    fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
        where S: serde::Serializer
    {
        self.0.serialize(ser)
    }
}

impl<'de: 'a, 'a> serde::Deserialize<'de> for &'a Key {
    fn deserialize<D>(de: D) -> Result<Self, D::Error>
        where D: serde::Deserializer<'de>
    {
        <&'a str as serde::Deserialize<'de>>::deserialize(de).map(Key::new)
    }
}

impl<I: core::slice::SliceIndex<str, Output=str>> core::ops::Index<I> for Key {
    type Output = Key;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        self.0[index].into()
    }
}

impl PartialEq<str> for Key {
    fn eq(&self, other: &str) -> bool {
        self == Key::new(other)
    }
}

impl PartialEq<Key> for str {
    fn eq(&self, other: &Key) -> bool {
        Key::new(self) == other
    }
}

impl<'a, S: AsRef<str> + ?Sized> From<&'a S> for &'a Key {
    #[inline]
    fn from(string: &'a S) -> Self {
        Key::new(string)
    }
}

impl AsRef<Key> for str {
    fn as_ref(&self) -> &Key {
        Key::new(self)
    }
}

impl AsRef<Key> for RawStr {
    fn as_ref(&self) -> &Key {
        Key::new(self)
    }
}

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
