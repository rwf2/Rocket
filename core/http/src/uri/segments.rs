use std::path::PathBuf;
use std::str::Utf8Error;

use uri::Uri;

/// Iterator over the segments of an absolute URI path. Skips empty segments.
///
/// ### Examples
///
/// ```rust
/// # extern crate rocket;
/// use rocket::http::uri::Origin;
///
/// let uri = Origin::parse("/a/////b/c////////d").unwrap();
/// let segments = uri.segments();
/// for (i, segment) in segments.enumerate() {
///     match i {
///         0 => assert_eq!(segment, "a"),
///         1 => assert_eq!(segment, "b"),
///         2 => assert_eq!(segment, "c"),
///         3 => assert_eq!(segment, "d"),
///         _ => panic!("only four segments")
///     }
/// }
/// ```
#[derive(Clone, Debug)]
pub struct Segments<'a>(pub &'a str);

/// Errors which can occur when attempting to interpret a segment string as a
/// valid path segment.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SegmentError {
    /// The segment contained invalid UTF8 characters when percent decoded.
    Utf8(Utf8Error),
    /// The segment started with the wrapped invalid character.
    BadStart(char),
    /// The segment contained the wrapped invalid character.
    BadChar(char),
    /// The segment ended with the wrapped invalid character.
    BadEnd(char),
}

impl<'a> Segments<'a> {
    /// Creates a `PathBuf` from a `Segments` iterator. The returned `PathBuf`
    /// is percent-decoded. If a segment is equal to "..", the previous segment
    /// (if any) is skipped.
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
    pub fn into_path_buf(self, allow_dotfiles: bool) -> Result<PathBuf, SegmentError> {
        let mut buf = PathBuf::new();
        for segment in self {
            let decoded = Uri::percent_decode(segment.as_bytes())
                .map_err(SegmentError::Utf8)?;

            if decoded == ".." {
                buf.pop();
            } else if !allow_dotfiles && decoded.starts_with('.') {
                return Err(SegmentError::BadStart('.'))
            } else if decoded.starts_with('*') {
                return Err(SegmentError::BadStart('*'))
            } else if decoded.ends_with(':') {
                return Err(SegmentError::BadEnd(':'))
            } else if decoded.ends_with('>') {
                return Err(SegmentError::BadEnd('>'))
            } else if decoded.ends_with('<') {
                return Err(SegmentError::BadEnd('<'))
            } else if decoded.contains('/') {
                return Err(SegmentError::BadChar('/'))
            } else if cfg!(windows) && decoded.contains('\\') {
                return Err(SegmentError::BadChar('\\'))
            } else {
                buf.push(&*decoded)
            }
        }

        Ok(buf)
    }
}

impl<'a> Iterator for Segments<'a> {
    type Item = &'a str;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // Find the start of the next segment (first that's not '/').
        let i = self.0.find(|c| c != '/')?;

        // Get the index of the first character that _is_ a '/' after start.
        // j = index of first character after i (hence the i +) that's not a '/'
        let j = self.0[i..].find('/').map_or(self.0.len(), |j| i + j);

        // Save the result, update the iterator, and return!
        let result = Some(&self.0[i..j]);
        self.0 = &self.0[j..];
        result
    }

    // TODO: Potentially take a second parameter with Option<cached count> and
    // return it here if it's Some. The downside is that a decision has to be
    // made about -when- to compute and cache that count. A place to do it is in
    // the segments() method. But this means that the count will always be
    // computed regardless of whether it's needed. Maybe this is ok. We'll see.
    // fn count(self) -> usize where Self: Sized {
    //     self.1.unwrap_or_else(self.fold(0, |cnt, _| cnt + 1))
    // }
}
