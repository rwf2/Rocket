use std::path::PathBuf;

use crate::RawStr;
use crate::parse::IndexedStr;

/// Iterator over the non-empty, percent-decoded segments of a URI path.
///
/// Returned by [`Origin::path_segments()`].
///
/// ### Examples
///
/// ```rust
/// # extern crate rocket;
/// use rocket::http::uri::Origin;
///
/// let uri = Origin::parse("/a%20z/////b/c////////d").unwrap();
/// let segments = uri.path_segments();
/// for (i, segment) in segments.enumerate() {
///     match i {
///         0 => assert_eq!(segment, "a z"),
///         1 => assert_eq!(segment, "b"),
///         2 => assert_eq!(segment, "c"),
///         3 => assert_eq!(segment, "d"),
///         _ => panic!("only four segments")
///     }
/// }
/// # assert_eq!(uri.path_segments().len(), 4);
/// # assert_eq!(uri.path_segments().count(), 4);
/// # assert_eq!(uri.path_segments().next(), Some("a z"));
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Segments<'o> {
    pub(super) source: &'o RawStr,
    pub(super) segments: &'o [IndexedStr<'static>],
    pub(super) pos: usize,
}

/// An error interpreting a segment as a [`PathBuf`] component in
/// [`Segments::to_path_buf()`].
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum PathError {
    /// The segment started with the wrapped invalid character.
    BadStart(char),
    /// The segment contained the wrapped invalid character.
    BadChar(char),
    /// The segment ended with the wrapped invalid character.
    BadEnd(char),
}

impl<'o> Segments<'o> {
    /// Returns the number of path segments left.
    #[inline]
    pub fn len(&self) -> usize {
        let max_pos = std::cmp::min(self.pos, self.segments.len());
        self.segments.len() - max_pos
    }

    /// Returns `true` if there are no segments left.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Skips `n` segments.
    #[inline]
    pub fn skip(mut self, n: usize) -> Self {
        self.pos = std::cmp::min(self.pos + n, self.segments.len());
        self
    }

    /// Get the `n`th segment from the current position.
    #[inline]
    pub fn get(&self, n: usize) -> Option<&'o str> {
        self.segments.get(self.pos + n)
            .map(|i| i.from_source(Some(self.source.as_str())))
    }

    /// Returns `true` if `self` is a prefix of `other`.
    #[inline]
    pub fn prefix_of<'b>(self, other: Segments<'b>) -> bool {
        if self.len() > other.len() {
            return false;
        }

        self.zip(other).all(|(a, b)| a == b)
    }

    /// Creates a `PathBuf` from `self`. The returned `PathBuf` is
    /// percent-decoded. If a segment is equal to "..", the previous segment (if
    /// any) is skipped.
    ///
    /// For security purposes, if a segment meets any of the following
    /// conditions, an `Err` is returned indicating the condition met:
    ///
    ///   * Decoded segment starts with any of: '*'
    ///   * Decoded segment ends with any of: `:`, `>`, `<`
    ///   * Decoded segment contains any of: `/`
    ///   * On Windows, decoded segment contains any of: `\`
    ///   * Percent-encoding results in invalid UTF8.
    ///
    /// Additionally, if `allow_dotfiles` is `false`, an `Err` is returned if
    /// the following condition is met:
    ///
    ///   * Decoded segment starts with any of: `.` (except `..`)
    ///
    /// As a result of these conditions, a `PathBuf` derived via `FromSegments`
    /// is safe to interpolate within, or use as a suffix of, a path without
    /// additional checks.
    pub fn to_path_buf(&self, allow_dotfiles: bool) -> Result<PathBuf, PathError> {
        let mut buf = PathBuf::new();
        for segment in *self {
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

        Ok(buf)
    }
}

impl<'o> Iterator for Segments<'o> {
    type Item = &'o str;

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

/// Decoded query segments iterator.
#[derive(Debug, Clone, Copy)]
pub struct QuerySegments<'o> {
    pub(super) source: Option<&'o RawStr>,
    pub(super) segments: &'o [(IndexedStr<'static>, IndexedStr<'static>)],
    pub(super) pos: usize,
}

impl<'o> QuerySegments<'o> {
    /// Check if there are no query segments left.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of query segments left.
    pub fn len(&self) -> usize {
        let max_pos = std::cmp::min(self.pos, self.segments.len());
        self.segments.len() - max_pos
    }

    /// Skip `n` segments.
    pub fn skip(mut self, n: usize) -> Self {
        self.pos = std::cmp::min(self.pos + n, self.segments.len());
        self
    }

    /// Get the `n`th segment from the current position.
    #[inline]
    pub fn get(&self, n: usize) -> Option<(&'o str, &'o str)> {
        let (name, val) = self.segments.get(self.pos + n)?;
        let source = self.source.map(|s| s.as_str());
        let name = name.from_source(source);
        let val = val.from_source(source);
        Some((name, val))
    }
}

impl<'o> Iterator for QuerySegments<'o> {
    type Item = (&'o str, &'o str);

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
