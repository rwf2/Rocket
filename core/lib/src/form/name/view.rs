use crate::form::name::*;

/// A sliding-prefix view into a [`Name`].
///
/// A [`NameView`] maintains a sliding key view into a [`Name`]. The current key
/// ([`key()`]) can be [`shift()`ed](NameView::shift()) one key to the right.
/// The `Name` prefix including the current key can be extracted via
/// [`as_name()`] and the prefix _not_ including the current key via
/// [`parent()`].
///
/// [`key()`]: NameView::key()
/// [`as_name()`]: NameView::as_name()
/// [`parent()`]: NameView::parent()
///
/// This is best illustrated via an example:
///
/// ```rust
/// use rocket::form::name::NameView;
///
/// // The view begins at the first key. Illustrated: `(a).b[c:d]` where
/// // parenthesis enclose the current key.
/// let mut view = NameView::new("a.b[c:d]");
/// assert_eq!(view.key().unwrap(), "a");
/// assert_eq!(view.as_name(), "a");
/// assert_eq!(view.parent(), None);
///
/// // Shifted once to the right views the second key: `a.(b)[c:d]`.
/// view.shift();
/// assert_eq!(view.key().unwrap(), "b");
/// assert_eq!(view.as_name(), "a.b");
/// assert_eq!(view.parent().unwrap(), "a");
///
/// // Shifting again now has predictable results: `a.b[(c:d)]`.
/// view.shift();
/// assert_eq!(view.key().unwrap(), "c:d");
/// assert_eq!(view.as_name(), "a.b[c:d]");
/// assert_eq!(view.parent().unwrap(), "a.b");
///
/// // Shifting past the end means we have no further keys.
/// view.shift();
/// assert_eq!(view.key(), None);
/// assert_eq!(view.key_lossy(), "");
/// assert_eq!(view.as_name(), "a.b[c:d]");
/// assert_eq!(view.parent().unwrap(), "a.b[c:d]");
///
/// view.shift();
/// assert_eq!(view.key(), None);
/// assert_eq!(view.as_name(), "a.b[c:d]");
/// assert_eq!(view.parent().unwrap(), "a.b[c:d]");
/// ```
///
/// # Equality
///
/// `PartialEq`, `Eq`, and `Hash` all operate on the name prefix including the
/// current key. Only key values are compared; delimiters are insignificant.
/// Again, illustrated via examples:
///
/// ```rust
/// use rocket::form::name::NameView;
///
/// let mut view = NameView::new("a.b[c:d]");
/// assert_eq!(view, "a");
///
/// // Shifted once to the right views the second key: `a.(b)[c:d]`.
/// view.shift();
/// assert_eq!(view.key().unwrap(), "b");
/// assert_eq!(view.as_name(), "a.b");
/// assert_eq!(view, "a.b");
/// assert_eq!(view, "a[b]");
///
/// // Shifting again now has predictable results: `a.b[(c:d)]`.
/// view.shift();
/// assert_eq!(view, "a.b[c:d]");
/// assert_eq!(view, "a.b.c:d");
/// assert_eq!(view, "a[b].c:d");
/// assert_eq!(view, "a[b]c:d");
/// ```
#[derive(Copy, Clone)]
pub struct NameView<'v> {
    name: &'v Name,
    start: usize,
    end: usize,
}

impl<'v> NameView<'v> {
    /// Initializes a new `NameView` at the first key of `name`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::form::name::NameView;
    ///
    /// let mut view = NameView::new("a.b[c:d]");
    /// assert_eq!(view.key().unwrap(), "a");
    /// assert_eq!(view.as_name(), "a");
    /// assert_eq!(view.parent(), None);
    /// ```
    pub fn new<N: Into<&'v Name>>(name: N) -> Self {
        let mut view = NameView { name: name.into(), start: 0, end: 0 };
        view.shift();
        view
    }

    /// Shifts the current key once to the right.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rocket::form::name::NameView;
    ///
    /// let mut view = NameView::new("a.b[c:d][d.e]");
    /// assert_eq!(view.key().unwrap(), "a");
    ///
    /// view.shift();
    /// assert_eq!(view.key().unwrap(), "b");
    ///
    /// view.shift();
    /// assert_eq!(view.key().unwrap(), "c:d");
    ///
    /// view.shift();
    /// assert_eq!(view.key().unwrap(), "d.e");
    /// ```
    ///
    /// Malformed strings can have interesting results:
    ///
    /// ```rust
    /// use rocket::form::name::NameView;
    ///
    /// let mut view = NameView::new("a[c.d");
    /// assert_eq!(view.key_lossy(), "a");
    ///
    /// view.shift();
    /// assert_eq!(view.key_lossy(), "c.d");
    ///
    /// let mut view = NameView::new("a[c[.d]");
    /// assert_eq!(view.key_lossy(), "a");
    ///
    /// view.shift();
    /// assert_eq!(view.key_lossy(), "c[.d");
    ///
    /// view.shift();
    /// assert_eq!(view.key(), None);
    ///
    /// let mut view = NameView::new("foo[c[.d]]");
    /// assert_eq!(view.key_lossy(), "foo");
    ///
    /// view.shift();
    /// assert_eq!(view.key_lossy(), "c[.d");
    ///
    /// view.shift();
    /// assert_eq!(view.key_lossy(), "]");
    ///
    /// view.shift();
    /// assert_eq!(view.key(), None);
    /// ```
    pub fn shift(&mut self) {
        const START_DELIMS: &[char] = &['.', '['];

        let string = &self.name[self.end..];
        let bytes = string.as_bytes();
        let shift = match bytes.get(0) {
            None | Some(b'=') => 0,
            Some(b'[') => match memchr::memchr(b']', bytes) {
                Some(j) => j + 1,
                None => bytes.len(),
            },
            Some(b'.') => match string[1..].find(START_DELIMS) {
                Some(j) => j + 1,
                None => bytes.len(),
            },
            _ => match string.find(START_DELIMS) {
                Some(j) => j,
                None => bytes.len()
            }
        };

        debug_assert!(self.end + shift <= self.name.len());
        *self = NameView {
            name: self.name,
            start: self.end,
            end: self.end + shift,
        };
    }

    /// Returns the key currently viewed by `self` if it is non-empty.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::form::name::NameView;
    ///
    /// let mut view = NameView::new("a[b]");
    /// assert_eq!(view.key().unwrap(), "a");
    ///
    /// view.shift();
    /// assert_eq!(view.key().unwrap(), "b");
    ///
    /// view.shift();
    /// assert_eq!(view.key(), None);
    /// # view.shift(); assert_eq!(view.key(), None);
    /// # view.shift(); assert_eq!(view.key(), None);
    /// ```
    pub fn key(&self) -> Option<&'v Key> {
        let lossy_key = self.key_lossy();
        if lossy_key.is_empty() {
            return None;
        }

        Some(lossy_key)
    }

    /// Returns the key currently viewed by `self`, even if it is non-empty.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::form::name::NameView;
    ///
    /// let mut view = NameView::new("a[b]");
    /// assert_eq!(view.key_lossy(), "a");
    ///
    /// view.shift();
    /// assert_eq!(view.key_lossy(), "b");
    ///
    /// view.shift();
    /// assert_eq!(view.key_lossy(), "");
    /// # view.shift(); assert_eq!(view.key_lossy(), "");
    /// # view.shift(); assert_eq!(view.key_lossy(), "");
    /// ```
    pub fn key_lossy(&self) -> &'v Key {
        let view = &self.name[self.start..self.end];
        let key = match view.as_bytes().get(0) {
            Some(b'.') => &view[1..],
            Some(b'[') if view.ends_with(']') => &view[1..view.len() - 1],
            Some(b'[') if self.is_at_last() => &view[1..],
            _ => view
        };

        key.as_str().into()
    }

    /// Returns the `Name` _up to and including_ the current key.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::form::name::NameView;
    ///
    /// let mut view = NameView::new("a[b]");
    /// assert_eq!(view.as_name(), "a");
    ///
    /// view.shift();
    /// assert_eq!(view.as_name(), "a[b]");
    /// # view.shift(); assert_eq!(view.as_name(), "a[b]");
    /// # view.shift(); assert_eq!(view.as_name(), "a[b]");
    /// ```
    pub fn as_name(&self) -> &'v Name {
        &self.name[..self.end]
    }

    /// Returns the `Name` _prior to_ the current key.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::form::name::NameView;
    ///
    /// let mut view = NameView::new("a[b]");
    /// assert_eq!(view.parent(), None);
    ///
    /// view.shift();
    /// assert_eq!(view.parent().unwrap(), "a");
    ///
    /// view.shift();
    /// assert_eq!(view.parent().unwrap(), "a[b]");
    /// # view.shift(); assert_eq!(view.parent().unwrap(), "a[b]");
    /// # view.shift(); assert_eq!(view.parent().unwrap(), "a[b]");
    /// ```
    pub fn parent(&self) -> Option<&'v Name> {
        if self.start > 0 {
            Some(&self.name[..self.start])
        } else {
            None
        }
    }

    /// Returns the underlying `Name`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::form::name::NameView;
    ///
    /// let mut view = NameView::new("a[b]");
    /// assert_eq!(view.source(), "a[b]");
    ///
    /// view.shift();
    /// assert_eq!(view.source(), "a[b]");
    ///
    /// view.shift();
    /// assert_eq!(view.source(), "a[b]");
    ///
    /// # view.shift(); assert_eq!(view.source(), "a[b]");
    /// # view.shift(); assert_eq!(view.source(), "a[b]");
    /// ```
    pub fn source(&self) -> &'v Name {
        self.name
    }

    // This is the last key. The next `shift()` will exhaust `self`.
    fn is_at_last(&self) -> bool {
        self.end == self.name.len()
    }

    // There are no more keys. A `shift` will do nothing.
    pub(crate) fn exhausted(&self) -> bool {
        self.start == self.name.len()
    }
}

impl std::fmt::Debug for NameView<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_name().fmt(f)
    }
}

impl std::fmt::Display for NameView<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_name().fmt(f)
    }
}

impl<'a, 'b> PartialEq<NameView<'b>> for NameView<'a> {
    fn eq(&self, other: &NameView<'b>) -> bool {
        self.as_name() == other.as_name()
    }
}

impl<B: PartialEq<Name>> PartialEq<B> for NameView<'_> {
    fn eq(&self, other: &B) -> bool {
        other == self.as_name()
    }
}

impl Eq for NameView<'_> {  }

impl std::hash::Hash for NameView<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_name().hash(state)
    }
}

impl std::borrow::Borrow<Name> for NameView<'_> {
    fn borrow(&self) -> &Name {
        self.as_name()
    }
}
