use std::fmt::{self, Display};
use std::borrow::Cow;

use crate::ext::IntoOwned;
use crate::parse::{Indexed, IndexedStr};
use crate::uri::{as_utf8_unchecked, Error, Segments};

use state::Storage;

/// A URI with an absolute path and optional query: `/path?query`.
///
/// Origin URIs are the primary type of URI encountered in Rocket applications.
/// They are also the _simplest_ type of URIs, made up of only a path and an
/// optional query.
///
/// # Structure
///
/// The following diagram illustrates the syntactic structure of an origin URI:
///
/// ```text
/// /first_segment/second_segment/third?optional=query
/// |---------------------------------| |------------|
///                 path                    query
/// ```
///
/// The URI must begin with a `/`, can be followed by any number of _segments_,
/// and an optional `?` query separator and query string.
///
/// # Normalization
///
/// Rocket prefers, and will sometimes require, origin URIs to be _normalized_.
/// A normalized origin URI is a valid origin URI that contains zero empty
/// segments except when there are no segments.
///
/// As an example, the following URIs are all valid, normalized URIs:
///
/// ```rust
/// # extern crate rocket;
/// # use rocket::http::uri::Origin;
/// # let valid_uris = [
/// "/",
/// "/a/b/c",
/// "/a/b/c?q",
/// "/some%20thing"
/// # ];
/// # for uri in &valid_uris {
/// #   assert!(Origin::parse(uri).unwrap().is_normalized());
/// # }
/// ```
///
/// By contrast, the following are valid but _abnormal_ URIs:
///
/// ```rust
/// # extern crate rocket;
/// # use rocket::http::uri::Origin;
/// # let invalid = [
/// "//",               // one empty segment
/// "/a/b/",            // trailing empty segment
/// "/a/ab//c//d"       // two empty segments
/// # ];
/// # for uri in &invalid {
/// #   assert!(!Origin::parse(uri).unwrap().is_normalized());
/// # }
/// ```
///
/// The [`Origin::to_normalized()`](crate::uri::Origin::to_normalized()) method can be
/// used to normalize any `Origin`:
///
/// ```rust
/// # extern crate rocket;
/// # use rocket::http::uri::Origin;
/// # let invalid = [
/// // abnormal versions
/// "//", "/a/b/", "/a/ab//c//d"
/// # ,
///
/// // normalized versions
/// "/",  "/a/b",  "/a/ab/c/d"
/// # ];
/// # for i in 0..(invalid.len() / 2) {
/// #     let abnormal = Origin::parse(invalid[i]).unwrap();
/// #     let expected = Origin::parse(invalid[i + (invalid.len() / 2)]).unwrap();
/// #     assert_eq!(abnormal.to_normalized(), expected);
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct Origin<'a> {
    crate source: Option<Cow<'a, str>>,
    crate path: IndexedStr<'a>,
    crate query: Option<IndexedStr<'a>>,
    crate segment_count: Storage<usize>,
}

impl<'b> PartialEq<Origin<'b>> for Origin<'_> {
    fn eq(&self, other: &Origin<'b>) -> bool {
        self.path() == other.path() && self.query() == other.query()
    }
}

impl IntoOwned for Origin<'_> {
    type Owned = Origin<'static>;

    fn into_owned(self) -> Origin<'static> {
        Origin {
            source: self.source.into_owned(),
            path: self.path.into_owned(),
            query: self.query.into_owned(),
            segment_count: self.segment_count
        }
    }
}

impl<'a> Origin<'a> {
    #[inline]
    crate unsafe fn raw(
        source: Cow<'a, [u8]>,
        path: Indexed<'a, [u8]>,
        query: Option<Indexed<'a, [u8]>>
    ) -> Origin<'a> {
        Origin {
            source: Some(as_utf8_unchecked(source)),
            path: path.coerce(),
            query: query.map(|q| q.coerce()),
            segment_count: Storage::new()
        }
    }

    // Used mostly for testing and to construct known good URIs from other parts
    // of Rocket. This should _really_ not be used outside of Rocket because the
    // resulting `Origin's` are not guaranteed to be valid origin URIs!
    #[doc(hidden)]
    pub fn new<P, Q>(path: P, query: Option<Q>) -> Origin<'a>
        where P: Into<Cow<'a, str>>, Q: Into<Cow<'a, str>>
    {
        Origin {
            source: None,
            path: Indexed::from(path),
            query: query.map(Indexed::from),
            segment_count: Storage::new()
        }
    }

    // Used to fabricate URIs in several places. Equivalent to `Origin::new("/",
    // None)` or `Origin::parse("/").unwrap()`. Should not be used outside of
    // Rocket, though doing so would be less harmful.
    #[doc(hidden)]
    pub fn dummy() -> Origin<'static> {
        Origin::new::<&'static str, &'static str>("/", None)
    }

    /// Parses the string `string` into an `Origin`. Parsing will never
    /// allocate. Returns an `Error` if `string` is not a valid origin URI.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Origin;
    ///
    /// // Parse a valid origin URI.
    /// let uri = Origin::parse("/a/b/c?query").expect("valid URI");
    /// assert_eq!(uri.path(), "/a/b/c");
    /// assert_eq!(uri.query(), Some("query"));
    ///
    /// // Invalid URIs fail to parse.
    /// Origin::parse("foo bar").expect_err("invalid URI");
    /// ```
    pub fn parse(string: &'a str) -> Result<Origin<'a>, Error<'a>> {
        crate::parse::uri::origin_from_str(string)
    }

    // Parses an `Origin` that may contain `<` or `>` characters which are
    // invalid according to the RFC but used by Rocket's routing URIs Don't use
    // this outside of Rocket!
    #[doc(hidden)]
    pub fn parse_route(string: &'a str) -> Result<Origin<'a>, Error<'a>> {
        crate::parse::uri::route_origin_from_str(string)
    }

    /// Parses the string `string` into an `Origin`. Parsing will never
    /// allocate. This method should be used instead of
    /// [`Origin::parse()`](crate::uri::Origin::parse()) when the source URI is already
    /// a `String`. Returns an `Error` if `string` is not a valid origin URI.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Origin;
    ///
    /// let source = format!("/foo/{}/three", 2);
    /// let uri = Origin::parse_owned(source).expect("valid URI");
    /// assert_eq!(uri.path(), "/foo/2/three");
    /// assert_eq!(uri.query(), None);
    /// ```
    pub fn parse_owned(string: String) -> Result<Origin<'static>, Error<'static>> {
        // We create a copy of a pointer to `string` to escape the borrow
        // checker. This is so that we can "move out of the borrow" later.
        //
        // For this to be correct and safe, we need to ensure that:
        //
        //  1. No `&mut` references to `string` are created after this line.
        //  2. `string` isn't dropped by `copy_of_str` is live.
        //
        // These two facts can be easily verified. An `&mut` can be created
        // because `string` isn't `mut`. Then, `string` is clearly not dropped
        // since it's passed in to `source`.
        let copy_of_str = unsafe { &*(string.as_str() as *const str) };
        let origin = Origin::parse(copy_of_str)?;

        let uri = match origin {
            Origin { source: Some(_), path, query, segment_count } => Origin {
                segment_count,
                path: path.into_owned(),
                query: query.into_owned(),
                // At this point, it's impossible for anything to be borrowing
                // `string` except for `source`, even though Rust doesn't know
                // it. Because we're replacing `source` here, there can't
                // possibly be a borrow remaining, it's safe to "move out of the
                // borrow".
                source: Some(Cow::Owned(string)),
            },
            _ => unreachable!("parser always parses with a source")
        };

        Ok(uri)
    }

    /// Returns `true` if `self` is normalized. Otherwise, returns `false`.
    ///
    /// See [Normalization](#normalization) for more information on what it
    /// means for an origin URI to be normalized.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Origin;
    ///
    /// let normal = Origin::parse("/").unwrap();
    /// assert!(normal.is_normalized());
    ///
    /// let normal = Origin::parse("/a/b/c").unwrap();
    /// assert!(normal.is_normalized());
    ///
    /// let abnormal = Origin::parse("/a/b/c//d").unwrap();
    /// assert!(!abnormal.is_normalized());
    /// ```
    pub fn is_normalized(&self) -> bool {
        self.path().starts_with('/') &&
            !self.path().contains("//") &&
            !(self.path().len() > 1 && self.path().ends_with('/'))
    }

    /// Normalizes `self`.
    ///
    /// See [Normalization](#normalization) for more information on what it
    /// means for an origin URI to be normalized.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Origin;
    ///
    /// let abnormal = Origin::parse("/a/b/c//d").unwrap();
    /// assert!(!abnormal.is_normalized());
    ///
    /// let normalized = abnormal.to_normalized();
    /// assert!(normalized.is_normalized());
    /// assert_eq!(normalized, Origin::parse("/a/b/c/d").unwrap());
    /// ```
    pub fn to_normalized(&self) -> Origin<'_> {
        if self.is_normalized() {
            Origin::new(self.path(), self.query())
        } else {
            let mut new_path = String::with_capacity(self.path().len());
            for segment in self.segments() {
                use std::fmt::Write;
                let _ = write!(new_path, "/{}", segment);
            }

            if new_path.is_empty() {
                new_path.push('/');
            }

            Origin::new(new_path, self.query())
        }
    }

    /// Returns the path part of this URI.
    ///
    /// ### Examples
    ///
    /// A URI with only a path:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Origin;
    ///
    /// let uri = Origin::parse("/a/b/c").unwrap();
    /// assert_eq!(uri.path(), "/a/b/c");
    /// ```
    ///
    /// A URI with a query:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Origin;
    ///
    /// let uri = Origin::parse("/a/b/c?name=bob").unwrap();
    /// assert_eq!(uri.path(), "/a/b/c");
    /// ```
    #[inline]
    pub fn path(&self) -> &str {
        self.path.from_cow_source(&self.source)
    }

    /// Returns the query part of this URI without the question mark, if there is
    /// any.
    ///
    /// ### Examples
    ///
    /// A URI with a query part:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Origin;
    ///
    /// let uri = Origin::parse("/a/b/c?alphabet=true").unwrap();
    /// assert_eq!(uri.query(), Some("alphabet=true"));
    /// ```
    ///
    /// A URI without the query part:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Origin;
    ///
    /// let uri = Origin::parse("/a/b/c").unwrap();
    /// assert_eq!(uri.query(), None);
    /// ```
    #[inline]
    pub fn query(&self) -> Option<&str> {
        self.query.as_ref().map(|q| q.from_cow_source(&self.source))
    }

    /// Removes the query part of this URI, if there is any.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Origin;
    ///
    /// let mut uri = Origin::parse("/a/b/c?query=some").unwrap();
    /// assert_eq!(uri.query(), Some("query=some"));
    ///
    /// uri.clear_query();
    /// assert_eq!(uri.query(), None);
    /// ```
    pub fn clear_query(&mut self) {
        self.query = None;
    }

    /// Returns an iterator over the segments of the path in this URI. Skips
    /// empty segments.
    ///
    /// ### Examples
    ///
    /// A valid URI with only non-empty segments:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Origin;
    ///
    /// let uri = Origin::parse("/a/b/c?a=true").unwrap();
    /// for (i, segment) in uri.segments().enumerate() {
    ///     match i {
    ///         0 => assert_eq!(segment, "a"),
    ///         1 => assert_eq!(segment, "b"),
    ///         2 => assert_eq!(segment, "c"),
    ///         _ => unreachable!("only three segments")
    ///     }
    /// }
    /// ```
    ///
    /// A URI with empty segments:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Origin;
    ///
    /// let uri = Origin::parse("///a//b///c////d?query&param").unwrap();
    /// for (i, segment) in uri.segments().enumerate() {
    ///     match i {
    ///         0 => assert_eq!(segment, "a"),
    ///         1 => assert_eq!(segment, "b"),
    ///         2 => assert_eq!(segment, "c"),
    ///         3 => assert_eq!(segment, "d"),
    ///         _ => unreachable!("only four segments")
    ///     }
    /// }
    /// ```
    #[inline(always)]
    pub fn segments(&self) -> Segments<'_> {
        Segments(self.path())
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
    /// # extern crate rocket;
    /// use rocket::http::uri::Origin;
    ///
    /// let uri = Origin::parse("/a/b/c").unwrap();
    /// assert_eq!(uri.segment_count(), 3);
    /// ```
    ///
    /// A URI with empty segments:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Origin;
    ///
    /// let uri = Origin::parse("/a/b//c/d///e").unwrap();
    /// assert_eq!(uri.segment_count(), 5);
    /// ```
    #[inline]
    pub fn segment_count(&self) -> usize {
        *self.segment_count.get_or_set(|| self.segments().count())
    }
}

impl Display for Origin<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path())?;
        if let Some(q) = self.query() {
            write!(f, "?{}", q)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Origin;

    fn seg_count(path: &str, expected: usize) -> bool {
        let actual = Origin::parse(path).unwrap().segment_count();
        if actual != expected {
            eprintln!("Count mismatch: expected {}, got {}.", expected, actual);
            eprintln!("{}", if actual != expected { "lifetime" } else { "buf" });
            eprintln!("Segments (for {}):", path);
            for (i, segment) in Origin::parse(path).unwrap().segments().enumerate() {
                eprintln!("{}: {}", i, segment);
            }
        }

        actual == expected
    }

    fn eq_segments(path: &str, expected: &[&str]) -> bool {
        let uri = match Origin::parse(path) {
            Ok(uri) => uri,
            Err(e) => panic!("failed to parse {}: {}", path, e)
        };

        let actual: Vec<&str> = uri.segments().collect();
        actual == expected
    }

    #[test]
    fn send_and_sync() {
        fn assert<T: Send + Sync>() {};
        assert::<Origin<'_>>();
    }

    #[test]
    fn simple_segment_count() {
        assert!(seg_count("/", 0));
        assert!(seg_count("/a", 1));
        assert!(seg_count("/a/", 1));
        assert!(seg_count("/a/", 1));
        assert!(seg_count("/a/b", 2));
        assert!(seg_count("/a/b/", 2));
        assert!(seg_count("/a/b/", 2));
        assert!(seg_count("/ab/", 1));
    }

    #[test]
    fn segment_count() {
        assert!(seg_count("////", 0));
        assert!(seg_count("//a//", 1));
        assert!(seg_count("//abc//", 1));
        assert!(seg_count("//abc/def/", 2));
        assert!(seg_count("//////abc///def//////////", 2));
        assert!(seg_count("/a/b/c/d/e/f/g", 7));
        assert!(seg_count("/a/b/c/d/e/f/g", 7));
        assert!(seg_count("/a/b/c/d/e/f/g/", 7));
        assert!(seg_count("/a/b/cdjflk/d/e/f/g", 7));
        assert!(seg_count("//aaflja/b/cdjflk/d/e/f/g", 7));
        assert!(seg_count("/a/b", 2));
    }

    #[test]
    fn single_segments_match() {
        assert!(eq_segments("/", &[]));
        assert!(eq_segments("/a", &["a"]));
        assert!(eq_segments("/a/", &["a"]));
        assert!(eq_segments("///a/", &["a"]));
        assert!(eq_segments("///a///////", &["a"]));
        assert!(eq_segments("/a///////", &["a"]));
        assert!(eq_segments("//a", &["a"]));
        assert!(eq_segments("/abc", &["abc"]));
        assert!(eq_segments("/abc/", &["abc"]));
        assert!(eq_segments("///abc/", &["abc"]));
        assert!(eq_segments("///abc///////", &["abc"]));
        assert!(eq_segments("/abc///////", &["abc"]));
        assert!(eq_segments("//abc", &["abc"]));
    }

    #[test]
    fn multi_segments_match() {
        assert!(eq_segments("/a/b/c", &["a", "b", "c"]));
        assert!(eq_segments("/a/b", &["a", "b"]));
        assert!(eq_segments("/a///b", &["a", "b"]));
        assert!(eq_segments("/a/b/c/d", &["a", "b", "c", "d"]));
        assert!(eq_segments("///a///////d////c", &["a", "d", "c"]));
        assert!(eq_segments("/abc/abc", &["abc", "abc"]));
        assert!(eq_segments("/abc/abc/", &["abc", "abc"]));
        assert!(eq_segments("///abc///////a", &["abc", "a"]));
        assert!(eq_segments("/////abc/b", &["abc", "b"]));
        assert!(eq_segments("//abc//c////////d", &["abc", "c", "d"]));
    }

    #[test]
    fn multi_segments_match_funky_chars() {
        assert!(eq_segments("/a/b/c!!!", &["a", "b", "c!!!"]));
    }

    #[test]
    fn segment_mismatch() {
        assert!(!eq_segments("/", &["a"]));
        assert!(!eq_segments("/a", &[]));
        assert!(!eq_segments("/a/a", &["a"]));
        assert!(!eq_segments("/a/b", &["b", "a"]));
        assert!(!eq_segments("/a/a/b", &["a", "b"]));
        assert!(!eq_segments("///a/", &[]));
    }

    fn test_query(uri: &str, query: Option<&str>) {
        let uri = Origin::parse(uri).unwrap();
        assert_eq!(uri.query(), query);
    }

    #[test]
    fn query_does_not_exist() {
        test_query("/test", None);
        test_query("/a/b/c/d/e", None);
        test_query("/////", None);
        test_query("//a///", None);
        test_query("/a/b/c", None);
        test_query("/", None);
    }

    #[test]
    fn query_exists() {
        test_query("/test?abc", Some("abc"));
        test_query("/a/b/c?abc", Some("abc"));
        test_query("/a/b/c/d/e/f/g/?abc", Some("abc"));
        test_query("/?123", Some("123"));
        test_query("/?", Some(""));
        test_query("/?", Some(""));
        test_query("/?hi", Some("hi"));
    }

    #[test]
    fn normalized() {
        let uri_to_string = |s| Origin::parse(s)
            .unwrap()
            .to_normalized()
            .to_string();

        assert_eq!(uri_to_string("/"), "/".to_string());
        assert_eq!(uri_to_string("//"), "/".to_string());
        assert_eq!(uri_to_string("//////a/"), "/a".to_string());
        assert_eq!(uri_to_string("//ab"), "/ab".to_string());
        assert_eq!(uri_to_string("//a"), "/a".to_string());
        assert_eq!(uri_to_string("/a/b///c"), "/a/b/c".to_string());
        assert_eq!(uri_to_string("/a///b/c/d///"), "/a/b/c/d".to_string());
    }
}
