use std::ops::Deref;

use ref_cast::RefCast;

use crate::http::RawStr;
use crate::form::name::*;

/// A field name composed of keys.
///
/// A form field name is composed of _keys_, delimited by `.` or `[]`. Keys, in
/// turn, are composed of _indices_, delimited by `:`. The graphic below
/// illustrates this composition for a single field in `$name=$value` format:
///
/// ```text
///       food.bart[bar:foo].blam[0_0][1000]=some-value
/// name  |--------------------------------|
/// key   |--| |--| |-----|  |--| |-|  |--|
/// index |--| |--| |-| |-|  |--| |-|  |--|
/// ```
///
/// A `Name` is a wrapper around the field name string with methods to easily
/// access its sub-components.
///
/// # Serialization
///
/// A value of this type is serialized exactly as an `&str` consisting of the
/// entire field name.
#[repr(transparent)]
#[derive(RefCast)]
pub struct Name(str);

impl Name {
    /// Wraps a string as a `Name`. This is cost-free.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::form::name::Name;
    ///
    /// let name = Name::new("a.b.c");
    /// assert_eq!(name.as_str(), "a.b.c");
    /// ```
    pub fn new<S: AsRef<str> + ?Sized>(string: &S) -> &Name {
        Name::ref_cast(string.as_ref())
    }

    /// Returns an iterator over the keys of `self`, including empty keys.
    ///
    /// See the [top-level docs](Self) for a description of "keys".
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::form::name::Name;
    ///
    /// let name = Name::new("apple.b[foo:bar]zoo.[barb].bat");
    /// let keys: Vec<_> = name.keys().map(|k| k.as_str()).collect();
    /// assert_eq!(keys, &["apple", "b", "foo:bar", "zoo", "", "barb", "bat"]);
    /// ```
    pub fn keys(&self) -> impl Iterator<Item = &Key> {
        struct Keys<'v>(NameView<'v>);

        impl<'v> Iterator for Keys<'v> {
            type Item = &'v Key;

            fn next(&mut self) -> Option<Self::Item> {
                if self.0.exhausted() {
                    return None;
                }

                let key = self.0.key_lossy();
                self.0.shift();
                Some(key)
            }
        }

        Keys(NameView::new(self))
    }

    /// Returns an iterator over overlapping name prefixes of `self`, each
    /// succeeding prefix containing one more key than the previous.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::form::name::Name;
    ///
    /// let name = Name::new("apple.b[foo:bar]");
    /// let prefixes: Vec<_> = name.prefixes().map(|p| p.as_str()).collect();
    /// assert_eq!(prefixes, &["apple", "apple.b", "apple.b[foo:bar]"]);
    ///
    /// let name = Name::new("a.b.[foo]");
    /// let prefixes: Vec<_> = name.prefixes().map(|p| p.as_str()).collect();
    /// assert_eq!(prefixes, &["a", "a.b", "a.b.", "a.b.[foo]"]);
    /// ```
    pub fn prefixes(&self) -> impl Iterator<Item = &Name> {
        struct Prefixes<'v>(NameView<'v>);

        impl<'v> Iterator for Prefixes<'v> {
            type Item = &'v Name;

            fn next(&mut self) -> Option<Self::Item> {
                if self.0.exhausted() {
                    return None;
                }

                let name = self.0.as_name();
                self.0.shift();
                Some(name)
            }
        }

        Prefixes(NameView::new(self))
    }

    /// Borrows the underlying string.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::form::name::Name;
    ///
    /// let name = Name::new("a.b.c");
    /// assert_eq!(name.as_str(), "a.b.c");
    /// ```
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl serde::Serialize for Name {
    fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
        where S: serde::Serializer
    {
        self.0.serialize(ser)
    }
}

impl<'de: 'a, 'a> serde::Deserialize<'de> for &'a Name {
    fn deserialize<D>(de: D) -> Result<Self, D::Error>
        where D: serde::Deserializer<'de>
    {
        <&'a str as serde::Deserialize<'de>>::deserialize(de).map(Name::new)
    }
}

impl<'a, S: AsRef<str> + ?Sized> From<&'a S> for &'a Name {
    #[inline]
    fn from(string: &'a S) -> Self {
        Name::new(string)
    }
}

impl Deref for Name {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<I: core::slice::SliceIndex<str, Output=str>> core::ops::Index<I> for Name {
    type Output = Name;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        self.0[index].into()
    }
}

impl PartialEq for Name {
    fn eq(&self, other: &Self) -> bool {
        self.keys().eq(other.keys())
    }
}

impl PartialEq<str> for Name {
    fn eq(&self, other: &str) -> bool {
        self == Name::new(other)
    }
}

impl PartialEq<Name> for str {
    fn eq(&self, other: &Name) -> bool {
        Name::new(self) == other
    }
}

impl PartialEq<&str> for Name {
    fn eq(&self, other: &&str) -> bool {
        self == Name::new(other)
    }
}

impl PartialEq<Name> for &str {
    fn eq(&self, other: &Name) -> bool {
        Name::new(self) == other
    }
}

impl AsRef<Name> for str {
    fn as_ref(&self) -> &Name {
        Name::new(self)
    }
}

impl AsRef<Name> for RawStr {
    fn as_ref(&self) -> &Name {
        Name::new(self)
    }
}

impl AsRef<Name> for Name {
    fn as_ref(&self) -> &Name {
        self
    }
}

impl Eq for Name { }

impl std::hash::Hash for Name {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.keys().for_each(|k| k.hash(state))
    }
}

impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::fmt::Debug for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
