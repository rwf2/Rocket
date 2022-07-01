use std::borrow::Cow;

use crate::form::name::*;

/// A potentially owned [`Name`].
///
/// Constructible from a [`NameView`], [`Name`], `&str`, or `String`, a
/// `NameBuf` acts much like a [`Name`] but can be converted into an owned
/// version via [`IntoOwned`](crate::http::ext::IntoOwned).
///
/// ```rust
/// use rocket::form::name::NameBuf;
/// use rocket::http::ext::IntoOwned;
///
/// let alloc = String::from("a.b.c");
/// let name = NameBuf::from(alloc.as_str());
/// let owned: NameBuf<'static> = name.into_owned();
/// ```
#[derive(Clone)]
pub struct NameBuf<'v> {
    left: &'v Name,
    right: Cow<'v, str>,
}

impl<'v> NameBuf<'v> {
    #[inline]
    fn split(&self) -> (&Name, &Name) {
        (self.left, Name::new(&self.right))
    }

    /// Returns an iterator over the keys of `self`, including empty keys.
    ///
    /// See [`Name`] for a description of "keys".
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::form::name::NameBuf;
    ///
    /// let name = NameBuf::from("apple.b[foo:bar]zoo.[barb].bat");
    /// let keys: Vec<_> = name.keys().map(|k| k.as_str()).collect();
    /// assert_eq!(keys, &["apple", "b", "foo:bar", "zoo", "", "barb", "bat"]);
    /// ```
    #[inline]
    pub fn keys(&self) -> impl Iterator<Item = &Key> {
        let (left, right) = self.split();
        left.keys().chain(right.keys())
    }

    /// Returns `true` if `self` is empty.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::form::name::NameBuf;
    ///
    /// let name = NameBuf::from("apple.b[foo:bar]zoo.[barb].bat");
    /// assert!(!name.is_empty());
    ///
    /// let name = NameBuf::from("");
    /// assert!(name.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        let (left, right) = self.split();
        left.is_empty() && right.is_empty()
    }
}

impl crate::http::ext::IntoOwned for NameBuf<'_> {
    type Owned = NameBuf<'static>;

    fn into_owned(self) -> Self::Owned {
        let right = match (self.left, self.right) {
            (l, Cow::Owned(r)) if l.is_empty() => Cow::Owned(r),
            (l, r) if l.is_empty() => r.to_string().into(),
            (l, r) if r.is_empty() => l.to_string().into(),
            (l, r) => format!("{}.{}", l, r).into(),
        };

        NameBuf { left: "".into(), right }
    }
}

impl serde::Serialize for NameBuf<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: serde::Serializer
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'v> From<NameView<'v>> for NameBuf<'v> {
    fn from(nv: NameView<'v>) -> Self {
        NameBuf { left: nv.as_name(), right: Cow::Borrowed("") }
    }
}

impl<'v> From<&'v Name> for NameBuf<'v> {
    fn from(name: &'v Name) -> Self {
        NameBuf { left: name, right: Cow::Borrowed("") }
    }
}

impl<'v> From<&'v str> for NameBuf<'v> {
    fn from(name: &'v str) -> Self {
        NameBuf::from((None, Cow::Borrowed(name)))
    }
}

impl<'v> From<String> for NameBuf<'v> {
    fn from(name: String) -> Self {
        NameBuf::from((None, Cow::Owned(name)))
    }
}

#[doc(hidden)]
impl<'v> From<(Option<&'v Name>, Cow<'v, str>)> for NameBuf<'v> {
    fn from((prefix, right): (Option<&'v Name>, Cow<'v, str>)) -> Self {
        match prefix {
            Some(left) => NameBuf { left, right },
            None => NameBuf { left: "".into(), right }
        }
    }
}

#[doc(hidden)]
impl<'v> From<(Option<&'v Name>, String)> for NameBuf<'v> {
    fn from((prefix, right): (Option<&'v Name>, String)) -> Self {
        match prefix {
            Some(left) => NameBuf { left, right: right.into() },
            None => NameBuf { left: "".into(), right: right.into() }
        }
    }
}

#[doc(hidden)]
impl<'v> From<(Option<&'v Name>, &'v str)> for NameBuf<'v> {
    fn from((prefix, suffix): (Option<&'v Name>, &'v str)) -> Self {
        NameBuf::from((prefix, Cow::Borrowed(suffix)))
    }
}

#[doc(hidden)]
impl<'v> From<(&'v Name, &'v str)> for NameBuf<'v> {
    fn from((prefix, suffix): (&'v Name, &'v str)) -> Self {
        NameBuf::from((Some(prefix), Cow::Borrowed(suffix)))
    }
}

impl std::fmt::Debug for NameBuf<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"")?;

        let (left, right) = self.split();
        if !left.is_empty() { write!(f, "{}", left.escape_debug())? }
        if !right.is_empty() {
            if !left.is_empty() { f.write_str(".")?; }
            write!(f, "{}", right.escape_debug())?;
        }

        write!(f, "\"")
    }
}

impl std::fmt::Display for NameBuf<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (left, right) = self.split();
        if !left.is_empty() { left.fmt(f)?; }
        if !right.is_empty() {
            if !left.is_empty() { f.write_str(".")?; }
            right.fmt(f)?;
        }

        Ok(())
    }
}

impl PartialEq for NameBuf<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.keys().eq(other.keys())
    }
}

impl<N: AsRef<Name> + ?Sized> PartialEq<N> for NameBuf<'_> {
    fn eq(&self, other: &N) -> bool {
        self.keys().eq(other.as_ref().keys())
    }
}

impl PartialEq<NameBuf<'_>> for Name {
    fn eq(&self, other: &NameBuf<'_>) -> bool {
        self.keys().eq(other.keys())
    }
}

impl PartialEq<NameBuf<'_>> for str {
    fn eq(&self, other: &NameBuf<'_>) -> bool {
        Name::new(self) == other
    }
}

impl PartialEq<NameBuf<'_>> for &str {
    fn eq(&self, other: &NameBuf<'_>) -> bool {
        Name::new(self) == other
    }
}

impl Eq for NameBuf<'_> { }

impl std::hash::Hash for NameBuf<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.keys().for_each(|k| k.hash(state))
    }
}

impl indexmap::Equivalent<Name> for NameBuf<'_> {
    fn equivalent(&self, key: &Name) -> bool {
        self.keys().eq(key.keys())
    }
}

impl indexmap::Equivalent<NameBuf<'_>> for Name {
    fn equivalent(&self, key: &NameBuf<'_>) -> bool {
        self.keys().eq(key.keys())
    }
}
