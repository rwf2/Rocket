use std::path::PathBuf;

use crate::RawStr;
use crate::uri::fmt::{Part, Path, Query};
use crate::uri::error::PathError;

/// Iterator over the non-empty, percent-decoded segments of a URI component.
///
/// Returned by [`Path::segments()`] and [`Query::segments()`].
///
/// [`Path::segments()`]: crate::uri::Path::segments()
/// [`Query::segments()`]: crate::uri::Query::segments()
///
/// # Example
///
/// ```rust
/// # extern crate rocket;
/// use rocket::http::uri::Origin;
///
/// let uri = Origin::parse("/a%20z/////b/c////////d").unwrap();
/// let segments = uri.path().segments();
/// for (i, segment) in segments.enumerate() {
///     match i {
///         0 => assert_eq!(segment, "a z"),
///         1 => assert_eq!(segment, "b"),
///         2 => assert_eq!(segment, "c"),
///         3 => assert_eq!(segment, "d"),
///         _ => panic!("only four segments")
///     }
/// }
/// # assert_eq!(uri.path().segments().len(), 4);
/// # assert_eq!(uri.path().segments().count(), 4);
/// # assert_eq!(uri.path().segments().next(), Some("a z"));
/// ```
#[derive(Debug, Clone)]
pub struct Segments<'a, P: Part> {
    pub(super) source: &'a RawStr,
    pub(super) segments: &'a [P::Raw],
    pub(super) pos: usize,
}

impl<P: Part> Segments<'_, P> {
    #[doc(hidden)]
    #[inline(always)]
    pub fn new<'a>(source: &'a RawStr, segments: &'a [P::Raw]) -> Segments<'a, P> {
        Segments { source, segments, pos: 0, }
    }

    /// Returns the number of path segments left.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// let uri = uri!("/foo/bar?baz&cat&car");
    ///
    /// let mut segments = uri.path().segments();
    /// assert_eq!(segments.len(), 2);
    ///
    /// segments.next();
    /// assert_eq!(segments.len(), 1);
    ///
    /// segments.next();
    /// assert_eq!(segments.len(), 0);
    ///
    /// segments.next();
    /// assert_eq!(segments.len(), 0);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        let max_pos = std::cmp::min(self.pos, self.segments.len());
        self.segments.len() - max_pos
    }

    /// Returns `true` if there are no segments left.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// let uri = uri!("/foo/bar?baz&cat&car");
    ///
    /// let mut segments = uri.path().segments();
    /// assert!(!segments.is_empty());
    ///
    /// segments.next();
    /// segments.next();
    /// assert!(segments.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a new `Segments` with `n` segments skipped.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// let uri = uri!("/foo/bar/baz/cat");
    ///
    /// let mut segments = uri.path().segments();
    /// assert_eq!(segments.len(), 4);
    /// assert_eq!(segments.next(), Some("foo"));
    ///
    /// let mut segments = segments.skip(2);
    /// assert_eq!(segments.len(), 1);
    /// assert_eq!(segments.next(), Some("cat"));
    /// ```
    #[inline]
    pub fn skip(mut self, n: usize) -> Self {
        self.pos = std::cmp::min(self.pos + n, self.segments.len());
        self
    }
}

impl<'a> Segments<'a, Path> {
    /// Returns the `n`th segment, 0-indexed, from the current position.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// let uri = uri!("/foo/bar/baaaz/cat");
    ///
    /// let segments = uri.path().segments();
    /// assert_eq!(segments.get(0), Some("foo"));
    /// assert_eq!(segments.get(1), Some("bar"));
    /// assert_eq!(segments.get(2), Some("baaaz"));
    /// assert_eq!(segments.get(3), Some("cat"));
    /// assert_eq!(segments.get(4), None);
    /// ```
    #[inline]
    pub fn get(&self, n: usize) -> Option<&'a str> {
        self.segments.get(self.pos + n)
            .map(|i| i.from_source(Some(self.source.as_str())))
    }

    /// Returns `true` if `self` is a prefix of `other`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// let a = uri!("/foo/bar/baaaz/cat");
    /// let b = uri!("/foo/bar");
    ///
    /// assert!(b.path().segments().prefix_of(a.path().segments()));
    /// assert!(!a.path().segments().prefix_of(b.path().segments()));
    ///
    /// let a = uri!("/foo/bar/baaaz/cat");
    /// let b = uri!("/dog/foo/bar");
    /// assert!(b.path().segments().skip(1).prefix_of(a.path().segments()));
    /// ```
    #[inline]
    pub fn prefix_of(self, other: Segments<'_, Path>) -> bool {
        if self.len() > other.len() {
            return false;
        }

        self.zip(other).all(|(a, b)| a == b)
    }

    /// Creates a `PathBuf` from `self`. The returned `PathBuf` is
    /// percent-decoded. If a segment is equal to `..`, the previous segment (if
    /// any) is skipped.
    ///
    /// For security purposes, if a segment meets any of the following
    /// conditions, an `Err` is returned indicating the condition met:
    ///
    ///   * Decoded segment starts with any of: `*`
    ///   * Decoded segment ends with any of: `:`, `>`, `<`
    ///   * Decoded segment contains any of: `/`
    ///   * On Windows, decoded segment contains any of: `\`
    ///   * Percent-encoding results in invalid UTF-8.
    ///
    /// Additionally, if `allow_dotfiles` is `false`, an `Err` is returned if
    /// the following condition is met:
    ///
    ///   * Decoded segment starts with any of: `.` (except `..`)
    ///
    /// As a result of these conditions, a `PathBuf` derived via `FromSegments`
    /// is safe to interpolate within, or use as a suffix of, a path without
    /// additional checks.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use std::path::Path;
    ///
    /// let uri = uri!("/a/b/c/d/.pass");
    ///
    /// let path = uri.path().segments().to_path_buf(true);
    /// assert_eq!(path.unwrap(), Path::new("a/b/c/d/.pass"));
    ///
    /// let path = uri.path().segments().to_path_buf(false);
    /// assert!(path.is_err());
    /// ```
    pub fn to_path_buf(&self, allow_dotfiles: bool) -> Result<PathBuf, PathError> {
        let mut buf = PathBuf::new();
        for segment in self.clone() {
            if segment == ".." {
                buf.pop();
            } else if !allow_dotfiles && segment.starts_with('.') {
                return Err(PathError::BadStart('.'))
            } else if segment.starts_with('*') {
                return Err(PathError::BadStart('*'))
            } else if segment.ends_with(':') {
                return Err(PathError::BadEnd(':'))
            } else if segment.ends_with('>') {
                return Err(PathError::BadEnd('>'))
            } else if segment.ends_with('<') {
                return Err(PathError::BadEnd('<'))
            } else if segment.contains('/') {
                return Err(PathError::BadChar('/'))
            } else if cfg!(windows) && segment.contains('\\') {
                return Err(PathError::BadChar('\\'))
            } else {
                buf.push(&*segment)
            }
        }

        // TODO: Should we check the filename against the list in `FileName`?
        // That list is mostly for writing, while this is likely to be read.
        // TODO: Add an option to allow/disallow shell characters?

        Ok(buf)
    }
}

impl<'a> Segments<'a, Query> {
    /// Returns the `n`th segment, 0-indexed, from the current position.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// let uri = uri!("/?foo=1&bar=hi+there&baaaz&cat=dog&=oh");
    ///
    /// let segments = uri.query().unwrap().segments();
    /// assert_eq!(segments.get(0), Some(("foo", "1")));
    /// assert_eq!(segments.get(1), Some(("bar", "hi there")));
    /// assert_eq!(segments.get(2), Some(("baaaz", "")));
    /// assert_eq!(segments.get(3), Some(("cat", "dog")));
    /// assert_eq!(segments.get(4), Some(("", "oh")));
    /// assert_eq!(segments.get(5), None);
    /// ```
    #[inline]
    pub fn get(&self, n: usize) -> Option<(&'a str, &'a str)> {
        let (name, val) = self.segments.get(self.pos + n)?;
        let source = Some(self.source.as_str());
        let name = name.from_source(source);
        let val = val.from_source(source);
        Some((name, val))
    }
}

macro_rules! impl_iterator {
    ($T:ty => $I:ty) => (
        impl<'a> Iterator for Segments<'a, $T> {
            type Item = $I;

            fn next(&mut self) -> Option<Self::Item> {
                let item = self.get(0)?;
                self.pos += 1;
                Some(item)
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                (self.len(), Some(self.len()))
            }

            fn count(self) -> usize {
                self.len()
            }
        }
    )
}

impl_iterator!(Path => &'a str);
impl_iterator!(Query => (&'a str, &'a str));
