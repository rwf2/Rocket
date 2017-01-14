//! Borrowed and owned string types for absolute URIs.

use std::cell::Cell;
use std::convert::From;
use std::fmt;
use std::borrow::Cow;
use std::str::Utf8Error;

use url;

use router::Collider;

/// Index (start, end) into a string, to prevent borrowing.
type Index = (usize, usize);

// TODO: Reconsider deriving PartialEq and Eq to make "//a/b" == "/a/b".
/// Borrowed string type for absolute URIs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct URI<'a> {
    uri: Cow<'a, str>,
    path: Index,
    query: Option<Index>,
    fragment: Option<Index>,
    segment_count: Cell<Option<usize>>,
}

impl<'a> URI<'a> {
    /// Constructs a new URI from a given string. The URI is assumed to be an
    /// absolute, well formed URI.
    pub fn new<T: Into<Cow<'a, str>>>(uri: T) -> URI<'a> {
        let uri = uri.into();
        let qmark = uri.find('?');
        let hmark = uri.find('#');

        let end = uri.len();
        let (path, query, fragment) = match (qmark, hmark) {
            (Some(i), Some(j)) => ((0, i), Some((i+1, j)), Some((j+1, end))),
            (Some(i), None) => ((0, i), Some((i+1, end)), None),
            (None, Some(j)) => ((0, j), None, Some((j+1, end))),
            (None, None) => ((0, end), None, None),
        };

        URI {
            uri: uri,
            path: path,
            query: query,
            fragment: fragment,
            segment_count: Cell::new(None),
        }
    }

    /// Returns the number of segments in the URI. Empty segments, which are
    /// invalid according to RFC#3986, are not counted.
    ///
    /// The segment count is cached after the first invocation. As a result,
    /// this function is O(1) after the first invocation, and O(n) before.
    ///
    /// ### Examples
    ///
    /// A valid URI with only non-empty segments:
    ///
    /// ```rust
    /// use rocket::http::uri::URI;
    ///
    /// let uri = URI::new("/a/b/c");
    /// assert_eq!(uri.segment_count(), 3);
    /// ```
    ///
    /// A URI with empty segments:
    ///
    /// ```rust
    /// use rocket::http::uri::URI;
    ///
    /// let uri = URI::new("/a/b//c/d///e");
    /// assert_eq!(uri.segment_count(), 5);
    /// ```
    #[inline(always)]
    pub fn segment_count(&self) -> usize {
        self.segment_count.get().unwrap_or_else(|| {
            let count = self.segments().count();
            self.segment_count.set(Some(count));
            count
        })
    }

    /// Returns an iterator over the segments of the path in this URI. Skips
    /// empty segments.
    ///
    /// ### Examples
    ///
    /// A valid URI with only non-empty segments:
    ///
    /// ```rust
    /// use rocket::http::uri::URI;
    ///
    /// let uri = URI::new("/a/b/c?a=true#done");
    /// for (i, segment) in uri.segments().enumerate() {
    ///     match i {
    ///         0 => assert_eq!(segment, "a"),
    ///         1 => assert_eq!(segment, "b"),
    ///         2 => assert_eq!(segment, "c"),
    ///         _ => panic!("only three segments")
    ///     }
    /// }
    /// ```
    ///
    /// A URI with empty segments:
    ///
    /// ```rust
    /// use rocket::http::uri::URI;
    ///
    /// let uri = URI::new("///a//b///c////d?#");
    /// for (i, segment) in uri.segments().enumerate() {
    ///     match i {
    ///         0 => assert_eq!(segment, "a"),
    ///         1 => assert_eq!(segment, "b"),
    ///         2 => assert_eq!(segment, "c"),
    ///         3 => assert_eq!(segment, "d"),
    ///         _ => panic!("only four segments")
    ///     }
    /// }
    /// ```
    #[inline(always)]
    pub fn segments(&self) -> Segments {
        Segments(self.path())
    }

    /// Returns the path part of this URI.
    ///
    /// ### Examples
    ///
    /// A URI with only a path:
    ///
    /// ```rust
    /// use rocket::http::uri::URI;
    ///
    /// let uri = URI::new("/a/b/c");
    /// assert_eq!(uri.path(), "/a/b/c");
    /// ```
    ///
    /// A URI with other components:
    ///
    /// ```rust
    /// use rocket::http::uri::URI;
    ///
    /// let uri = URI::new("/a/b/c?name=bob#done");
    /// assert_eq!(uri.path(), "/a/b/c");
    /// ```
    #[inline(always)]
    pub fn path(&self) -> &str {
        let (i, j) = self.path;
        &self.uri[i..j]
    }

    /// Returns the query part of this URI without the question mark, if there is
    /// any.
    ///
    /// ### Examples
    ///
    /// A URI with a query part:
    ///
    /// ```rust
    /// use rocket::http::uri::URI;
    ///
    /// let uri = URI::new("/a/b/c?alphabet=true");
    /// assert_eq!(uri.query(), Some("alphabet=true"));
    /// ```
    ///
    /// A URI without the query part:
    ///
    /// ```rust
    /// use rocket::http::uri::URI;
    ///
    /// let uri = URI::new("/a/b/c");
    /// assert_eq!(uri.query(), None);
    /// ```
    #[inline(always)]
    pub fn query(&self) -> Option<&str> {
        self.query.map(|(i, j)| &self.uri[i..j])
    }

    /// Returns the fargment part of this URI without the hash mark, if there is
    /// any.
    ///
    /// ### Examples
    ///
    /// A URI with a fragment part:
    ///
    /// ```rust
    /// use rocket::http::uri::URI;
    ///
    /// let uri = URI::new("/a?alphabet=true#end");
    /// assert_eq!(uri.fragment(), Some("end"));
    /// ```
    ///
    /// A URI without the fragment part:
    ///
    /// ```rust
    /// use rocket::http::uri::URI;
    ///
    /// let uri = URI::new("/a?query=true");
    /// assert_eq!(uri.fragment(), None);
    /// ```
    #[inline(always)]
    pub fn fragment(&self) -> Option<&str> {
        self.fragment.map(|(i, j)| &self.uri[i..j])
    }

    /// Returns a URL-decoded version of the string. If the percent encoded
    /// values are not valid UTF-8, an `Err` is returned.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rocket::http::uri::URI;
    ///
    /// let uri = URI::new("/Hello%2C%20world%21");
    /// let decoded_path = URI::percent_decode(uri.path().as_bytes()).expect("decoded");
    /// assert_eq!(decoded_path, "/Hello, world!");
    /// ```
    pub fn percent_decode(string: &[u8]) -> Result<Cow<str>, Utf8Error>  {
        let decoder = url::percent_encoding::percent_decode(string);
        decoder.decode_utf8()
    }

    /// Returns a URL-decoded version of the path. Any invalid UTF-8
    /// percent-encoded byte sequences will be replaced � U+FFFD, the
    /// replacement character.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rocket::http::uri::URI;
    ///
    /// let uri = URI::new("/Hello%2C%20world%21");
    /// let decoded_path = URI::percent_decode_lossy(uri.path().as_bytes());
    /// assert_eq!(decoded_path, "/Hello, world!");
    /// ```
    pub fn percent_decode_lossy(string: &[u8]) -> Cow<str> {
        let decoder = url::percent_encoding::percent_decode(string);
        decoder.decode_utf8_lossy()
    }

    /// Returns the inner string of this URI.
    ///
    /// The returned string is in raw form. It contains empty segments. If you'd
    /// like a string without empty segments, use `to_string` instead.
    ///
    /// ### Example
    ///
    /// ```rust
    /// use rocket::http::uri::URI;
    ///
    /// let uri = URI::new("/a/b///c/d/e//f?name=Mike#end");
    /// assert_eq!(uri.as_str(), "/a/b///c/d/e//f?name=Mike#end");
    /// ```
    #[inline(always)]
    pub fn as_str(&self) -> &str {
        &self.uri
    }
}

impl<'a> From<&'a str> for URI<'a> {
    #[inline(always)]
    fn from(uri: &'a str) -> URI<'a> {
        URI::new(uri)
    }
}

impl From<String> for URI<'static> {
    #[inline(always)]
    fn from(uri: String) -> URI<'static> {
        URI::new(uri)
    }
}

impl<'a> fmt::Display for URI<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // If this is the root path, then there are "zero" segments.
        if self.as_str().starts_with('/') && self.segment_count() == 0 {
            return write!(f, "/");
        }

        for segment in self.segments() {
            write!(f, "/{}", segment)?;
        }

        if let Some(query_str) = self.query() {
            write!(f, "?{}", query_str)?;
        }

        if let Some(fragment_str) = self.fragment() {
            write!(f, "#{}", fragment_str)?;
        }

        Ok(())
    }
}

unsafe impl<'a> Sync for URI<'a> { /* It's safe! */ }

impl<'a, 'b> Collider<URI<'b>> for URI<'a> {
    fn collides_with(&self, other: &URI<'b>) -> bool {
        for (seg_a, seg_b) in self.segments().zip(other.segments()) {
            if seg_a.ends_with("..>") || seg_b.ends_with("..>") {
                return true;
            }

            if !seg_a.collides_with(seg_b) {
                return false;
            }
        }

        if self.segment_count() != other.segment_count() {
            return false;
        }

        true
    }
}

/// Iterator over the segments of an absolute URI path. Skips empty segments.
///
/// ### Examples
///
/// ```rust
/// use rocket::http::uri::URI;
/// use rocket::http::uri::Segments;
///
/// let uri = URI::new("/a/////b/c////////d");
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

impl<'a> Iterator for Segments<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        // Find the start of the next segment (first that's not '/').
        let i = match self.0.find(|c| c != '/') {
            Some(index) => index,
            None => return None,
        };

        // Get the index of the first character that _is_ a '/' after start.
        // j = index of first character after i (hence the i +) that's not a '/'
        let rest = &self.0[i..];
        let j = rest.find('/').map_or(self.0.len(), |j| i + j);

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

#[cfg(test)]
mod tests {
    use super::URI;

    fn seg_count(path: &str, expected: usize) -> bool {
        let actual = URI::new(path).segment_count();
        if actual != expected {
            trace_!("Count mismatch: expected {}, got {}.", expected, actual);
            trace_!("{}", if actual != expected { "lifetime" } else { "buf" });
            trace_!("Segments (for {}):", path);
            for (i, segment) in URI::new(path).segments().enumerate() {
                trace_!("{}: {}", i, segment);
            }
        }

        actual == expected
    }

    fn eq_segments(path: &str, expected: &[&str]) -> bool {
        let uri = URI::new(path);
        let actual: Vec<&str> = uri.segments().collect();
        actual == expected
    }

    #[test]
    fn simple_segment_count() {
        assert!(seg_count("", 0));
        assert!(seg_count("/", 0));
        assert!(seg_count("a", 1));
        assert!(seg_count("/a", 1));
        assert!(seg_count("a/", 1));
        assert!(seg_count("/a/", 1));
        assert!(seg_count("/a/b", 2));
        assert!(seg_count("/a/b/", 2));
        assert!(seg_count("a/b/", 2));
        assert!(seg_count("ab/", 1));
    }

    #[test]
    fn segment_count() {
        assert!(seg_count("////", 0));
        assert!(seg_count("//a//", 1));
        assert!(seg_count("//abc//", 1));
        assert!(seg_count("//abc/def/", 2));
        assert!(seg_count("//////abc///def//////////", 2));
        assert!(seg_count("a/b/c/d/e/f/g", 7));
        assert!(seg_count("/a/b/c/d/e/f/g", 7));
        assert!(seg_count("/a/b/c/d/e/f/g/", 7));
        assert!(seg_count("/a/b/cdjflk/d/e/f/g", 7));
        assert!(seg_count("//aaflja/b/cdjflk/d/e/f/g", 7));
        assert!(seg_count("/a   /b", 2));
    }

    #[test]
    fn single_segments_match() {
        assert!(eq_segments("", &[]));
        assert!(eq_segments("a", &["a"]));
        assert!(eq_segments("/a", &["a"]));
        assert!(eq_segments("/a/", &["a"]));
        assert!(eq_segments("a/", &["a"]));
        assert!(eq_segments("///a/", &["a"]));
        assert!(eq_segments("///a///////", &["a"]));
        assert!(eq_segments("a///////", &["a"]));
        assert!(eq_segments("//a", &["a"]));
        assert!(eq_segments("", &[]));
        assert!(eq_segments("abc", &["abc"]));
        assert!(eq_segments("/a", &["a"]));
        assert!(eq_segments("/abc/", &["abc"]));
        assert!(eq_segments("abc/", &["abc"]));
        assert!(eq_segments("///abc/", &["abc"]));
        assert!(eq_segments("///abc///////", &["abc"]));
        assert!(eq_segments("abc///////", &["abc"]));
        assert!(eq_segments("//abc", &["abc"]));
    }

    #[test]
    fn multi_segments_match() {
        assert!(eq_segments("a/b/c", &["a", "b", "c"]));
        assert!(eq_segments("/a/b", &["a", "b"]));
        assert!(eq_segments("/a///b", &["a", "b"]));
        assert!(eq_segments("a/b/c/d", &["a", "b", "c", "d"]));
        assert!(eq_segments("///a///////d////c", &["a", "d", "c"]));
        assert!(eq_segments("abc/abc", &["abc", "abc"]));
        assert!(eq_segments("abc/abc/", &["abc", "abc"]));
        assert!(eq_segments("///abc///////a", &["abc", "a"]));
        assert!(eq_segments("/////abc/b", &["abc", "b"]));
        assert!(eq_segments("//abc//c////////d", &["abc", "c", "d"]));
    }

    #[test]
    fn multi_segments_match_funky_chars() {
        assert!(eq_segments("a/b/c!!!", &["a", "b", "c!!!"]));
        assert!(eq_segments("a  /b", &["a  ", "b"]));
        assert!(eq_segments("  a/b", &["  a", "b"]));
        assert!(eq_segments("  a/b  ", &["  a", "b  "]));
        assert!(eq_segments("  a///b  ", &["  a", "b  "]));
        assert!(eq_segments("  ab  ", &["  ab  "]));
    }

    #[test]
    fn segment_mismatch() {
        assert!(!eq_segments("", &["a"]));
        assert!(!eq_segments("a", &[]));
        assert!(!eq_segments("/a/a", &["a"]));
        assert!(!eq_segments("/a/b", &["b", "a"]));
        assert!(!eq_segments("/a/a/b", &["a", "b"]));
        assert!(!eq_segments("///a/", &[]));
    }

    fn test_query(uri: &str, query: Option<&str>) {
        let uri = URI::new(uri);
        assert_eq!(uri.query(), query);
    }

    fn test_fragment(uri: &str, fragment: Option<&str>) {
        let uri = URI::new(uri);
        assert_eq!(uri.fragment(), fragment);
    }

    #[test]
    fn query_does_not_exist() {
        test_query("/test", None);
        test_query("/a/b/c/d/e", None);
        test_query("/////", None);
        test_query("//a///", None);
    }

    #[test]
    fn query_exists() {
        test_query("/test?abc", Some("abc"));
        test_query("/a/b/c?abc", Some("abc"));
        test_query("/a/b/c/d/e/f/g/?abc#hijklmnop", Some("abc"));
        test_query("?123", Some("123"));
        test_query("?", Some(""));
        test_query("/?", Some(""));
        test_query("?#", Some(""));
        test_query("/?hi", Some("hi"));
    }

    #[test]
    fn fragment_exists() {
        test_fragment("/test#abc", Some("abc"));
        test_fragment("/#abc", Some("abc"));
        test_fragment("/a/b/c?123#a", Some("a"));
        test_fragment("/#a", Some("a"));
    }

    #[test]
    fn fragment_does_not_exist() {
        test_fragment("/testabc", None);
        test_fragment("/abc", None);
        test_fragment("/a/b/c?123", None);
        test_fragment("/a", None);
    }

    #[test]
    fn to_string() {
        let uri_to_string = |string| URI::new(string).to_string();

        assert_eq!(uri_to_string("/"), "/".to_string());
        assert_eq!(uri_to_string("//"), "/".to_string());
        assert_eq!(uri_to_string("//////a/"), "/a".to_string());
        assert_eq!(uri_to_string("//ab"), "/ab".to_string());
        assert_eq!(uri_to_string("//a"), "/a".to_string());
        assert_eq!(uri_to_string("/a/b///c"), "/a/b/c".to_string());
        assert_eq!(uri_to_string("/a///b/c/d///"), "/a/b/c/d".to_string());
    }
}
