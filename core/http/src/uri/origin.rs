use std::borrow::Cow;

use crate::ext::IntoOwned;
use crate::parse::{Extent, IndexedStr, uri::tables::is_pchar};
use crate::uri::{Error, Path, Query, Data, as_utf8_unchecked, fmt};
use crate::{RawStr, RawStrBuf};

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
/// "/hello?lang=en",
/// "/some%20thing?q=foo&lang=fr",
/// # ];
/// # for uri in &valid_uris {
/// #   assert!(Origin::parse(uri).unwrap().is_normalized());
/// # }
/// ```
///
/// By contrast, the following are valid but _non-normal_ URIs:
///
/// ```rust
/// # extern crate rocket;
/// # use rocket::http::uri::Origin;
/// # let invalid = [
/// "//",               // one empty segment
/// "/a/b/",            // trailing empty segment
/// "/a/ab//c//d",      // two empty segments
/// "/?a&&b",           // empty query segment
/// "/?foo&",           // trailing empty query segment
/// # ];
/// # for uri in &invalid {
/// #   assert!(!Origin::parse(uri).unwrap().is_normalized());
/// # }
/// ```
///
/// The [`Origin::into_normalized()`](crate::uri::Origin::into_normalized())
/// method can be used to normalize any `Origin`:
///
/// ```rust
/// # extern crate rocket;
/// # use rocket::http::uri::Origin;
/// # let invalid = [
/// // non-normal versions
/// "//", "/a/b/", "/a/ab//c//d", "/a?a&&b&",
///
/// // normalized versions
/// "/",  "/a/b",  "/a/ab/c/d", "/a?a&b",
/// # ];
/// # for i in 0..(invalid.len() / 2) {
/// #     let abnormal = Origin::parse(invalid[i]).unwrap();
/// #     let expected = Origin::parse(invalid[i + (invalid.len() / 2)]).unwrap();
/// #     assert_eq!(abnormal.into_normalized(), expected);
/// # }
/// ```
///
/// # (De)serialization
///
/// `Origin` is both `Serialize` and `Deserialize`:
///
/// ```rust
/// # #[cfg(feature = "serde")] mod serde {
/// # use serde_ as serde;
/// use serde::{Serialize, Deserialize};
/// use rocket::http::uri::Origin;
///
/// #[derive(Deserialize, Serialize)]
/// # #[serde(crate = "serde_")]
/// struct UriOwned {
///     uri: Origin<'static>,
/// }
///
/// #[derive(Deserialize, Serialize)]
/// # #[serde(crate = "serde_")]
/// struct UriBorrowed<'a> {
///     uri: Origin<'a>,
/// }
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Origin<'a> {
    pub(crate) source: Option<Cow<'a, str>>,
    pub(crate) path: Data<'a, fmt::Path>,
    pub(crate) query: Option<Data<'a, fmt::Query>>,
}

impl<'a> Origin<'a> {
    /// The root: `'/'`.
    #[doc(hidden)]
    pub const ROOT: Origin<'static> = Origin::const_new("/", None);

    /// SAFETY: `source` must be UTF-8.
    #[inline]
    pub(crate) unsafe fn raw(
        source: Cow<'a, [u8]>,
        path: Extent<&'a [u8]>,
        query: Option<Extent<&'a [u8]>>
    ) -> Origin<'a> {
        Origin {
            source: Some(as_utf8_unchecked(source)),
            path: Data::raw(path),
            query: query.map(Data::raw)
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
            path: Data::new(path.into()),
            query: query.map(Data::new),
        }
    }

    // Used mostly for testing and to construct known good URIs from other parts
    // of Rocket. This should _really_ not be used outside of Rocket because the
    // resulting `Origin's` are not guaranteed to be valid origin URIs!
    #[doc(hidden)]
    pub fn path_only<P: Into<Cow<'a, str>>>(path: P) -> Origin<'a> {
        Origin::new(path, None::<&'a str>)
    }

    // Used mostly for testing and to construct known good URIs from other parts
    // of Rocket. This should _really_ not be used outside of Rocket because the
    // resulting `Origin's` are not guaranteed to be valid origin URIs!
    #[doc(hidden)]
    pub const fn const_new(path: &'a str, query: Option<&'a str>) -> Origin<'a> {
        Origin {
            source: None,
            path: Data {
                value: IndexedStr::Concrete(Cow::Borrowed(path)),
                decoded_segments: state::InitCell::new(),
            },
            query: match query {
                Some(query) => Some(Data {
                    value: IndexedStr::Concrete(Cow::Borrowed(query)),
                    decoded_segments: state::InitCell::new(),
                }),
                None => None,
            },
        }
    }

    pub(crate) fn set_query<Q: Into<Option<Cow<'a, str>>>>(&mut self, query: Q) {
        self.query = query.into().map(Data::new);
    }

    /// Parses the string `string` into an `Origin`. Parsing will never
    /// allocate. Returns an `Error` if `string` is not a valid origin URI.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::uri::Origin;
    ///
    /// // Parse a valid origin URI.
    /// let uri = Origin::parse("/a/b/c?query").expect("valid URI");
    /// assert_eq!(uri.path(), "/a/b/c");
    /// assert_eq!(uri.query().unwrap(), "query");
    ///
    /// // Invalid URIs fail to parse.
    /// Origin::parse("foo bar").expect_err("invalid URI");
    ///
    /// // Prefer to use `uri!()` when the input is statically known:
    /// let uri = uri!("/a/b/c?query");
    /// assert_eq!(uri.path(), "/a/b/c");
    /// assert_eq!(uri.query().unwrap(), "query");
    /// ```
    pub fn parse(string: &'a str) -> Result<Origin<'a>, Error<'a>> {
        crate::parse::uri::origin_from_str(string)
    }

    // Parses an `Origin` which is allowed to contain _any_ `UTF-8` character.
    // The path must still be absolute `/..`. Don't use this outside of Rocket!
    #[doc(hidden)]
    pub fn parse_route(string: &'a str) -> Result<Origin<'a>, Error<'a>> {
        use pear::error::Expected;

        if !string.starts_with('/') {
            return Err(Error {
                expected: Expected::token(Some(&b'/'), string.as_bytes().get(0).cloned()),
                index: 0,
            });
        }

        let (path, query) = RawStr::new(string).split_at_byte(b'?');
        let query = (!query.is_empty()).then(|| query.as_str());
        Ok(Origin::new(path.as_str(), query))
    }

    /// Parses the string `string` into an `Origin`. Never allocates on success.
    /// May allocate on error.
    ///
    /// This method should be used instead of [`Origin::parse()`] when
    /// the source URI is already a `String`. Returns an `Error` if `string` is
    /// not a valid origin URI.
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
    /// assert!(uri.query().is_none());
    /// ```
    pub fn parse_owned(string: String) -> Result<Origin<'static>, Error<'static>> {
        let origin = Origin::parse(&string).map_err(|e| e.into_owned())?;
        debug_assert!(origin.source.is_some(), "Origin parsed w/o source");

        Ok(Origin {
            path: origin.path.into_owned(),
            query: origin.query.into_owned(),
            source: Some(Cow::Owned(string))
        })
    }

    /// Returns the path part of this URI.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// let uri = uri!("/a/b/c");
    /// assert_eq!(uri.path(), "/a/b/c");
    ///
    /// let uri = uri!("/a/b/c?name=bob");
    /// assert_eq!(uri.path(), "/a/b/c");
    /// ```
    #[inline]
    pub fn path(&self) -> Path<'_> {
        Path { source: &self.source, data: &self.path }
    }

    /// Returns the query part of this URI without the question mark, if there
    /// is any.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// let uri = uri!("/a/b/c?alphabet=true");
    /// assert_eq!(uri.query().unwrap(), "alphabet=true");
    ///
    /// let uri = uri!("/a/b/c");
    /// assert!(uri.query().is_none());
    /// ```
    #[inline]
    pub fn query(&self) -> Option<Query<'_>> {
        self.query.as_ref().map(|data| Query { source: &self.source, data })
    }

    /// Applies the function `f` to the internal `path` and returns a new
    /// `Origin` with the new path. If the path returned from `f` is invalid,
    /// returns `None`. Otherwise, returns `Some`, even if the new path is
    /// _abnormal_.
    ///
    /// ### Examples
    ///
    /// Affix a trailing slash if one isn't present.
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// let uri = uri!("/a/b/c");
    /// let expected_uri = uri!("/a/b/c/d");
    /// assert_eq!(uri.map_path(|p| format!("{}/d", p)), Some(expected_uri));
    ///
    /// let uri = uri!("/a/b/c");
    /// let abnormal_map = uri.map_path(|p| format!("{}///d", p));
    /// assert_eq!(abnormal_map.unwrap(), "/a/b/c///d");
    ///
    /// let uri = uri!("/a/b/c");
    /// let expected = uri!("/b/c");
    /// let mapped = uri.map_path(|p| p.strip_prefix("/a").unwrap_or(p));
    /// assert_eq!(mapped, Some(expected));
    ///
    /// let uri = uri!("/a");
    /// assert_eq!(uri.map_path(|p| p.strip_prefix("/a").unwrap_or(p)), None);
    ///
    /// let uri = uri!("/a/b/c");
    /// assert_eq!(uri.map_path(|p| format!("hi/{}", p)), None);
    /// ```
    #[inline]
    pub fn map_path<'s, F, P>(&'s self, f: F) -> Option<Self>
        where F: FnOnce(&'s RawStr) -> P, P: Into<RawStrBuf> + 's
    {
        let path = f(self.path().raw()).into();
        if !path.starts_with('/') || !path.as_bytes().iter().all(is_pchar) {
            return None;
        }

        Some(Origin {
            source: self.source.clone(),
            path: Data::new(Cow::from(path.into_string())),
            query: self.query.clone(),
        })
    }

    /// Removes the query part of this URI, if there is any.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// let mut uri = uri!("/a/b/c?query=some");
    /// assert_eq!(uri.query().unwrap(), "query=some");
    ///
    /// uri.clear_query();
    /// assert!(uri.query().is_none());
    /// ```
    pub fn clear_query(&mut self) {
        self.set_query(None);
    }

    /// Returns `true` if `self` is normalized. Otherwise, returns `false`.
    ///
    /// See [Normalization](Self#normalization) for more information on what it
    /// means for an origin URI to be normalized. Note that `uri!()` always
    /// normalizes static input.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::uri::Origin;
    ///
    /// assert!(Origin::parse("/").unwrap().is_normalized());
    /// assert!(Origin::parse("/a/b/c").unwrap().is_normalized());
    /// assert!(Origin::parse("/a/b/c?a=b&c").unwrap().is_normalized());
    ///
    /// assert!(!Origin::parse("/a/b/c//d").unwrap().is_normalized());
    /// assert!(!Origin::parse("/a?q&&b").unwrap().is_normalized());
    ///
    /// assert!(uri!("/a/b/c//d").is_normalized());
    /// assert!(uri!("/a?q&&b").is_normalized());
    /// ```
    pub fn is_normalized(&self) -> bool {
        self.path().is_normalized(true) && self.query().map_or(true, |q| q.is_normalized())
    }

    /// Normalizes `self`. This is a no-op if `self` is already normalized.
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
    /// let mut abnormal = Origin::parse("/a/b/c//d").unwrap();
    /// assert!(!abnormal.is_normalized());
    /// abnormal.normalize();
    /// assert!(abnormal.is_normalized());
    /// ```
    pub fn normalize(&mut self) {
        if !self.path().is_normalized(true) {
            self.path = self.path().to_normalized(true);
        }

        if let Some(query) = self.query() {
            if !query.is_normalized() {
                self.query = query.to_normalized();
            }
        }
    }

    /// Consumes `self` and returns a normalized version.
    ///
    /// This is a no-op if `self` is already normalized. See
    /// [Normalization](#normalization) for more information on what it means
    /// for an origin URI to be normalized.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Origin;
    ///
    /// let abnormal = Origin::parse("/a/b/c//d").unwrap();
    /// assert!(!abnormal.is_normalized());
    /// assert!(abnormal.into_normalized().is_normalized());
    /// ```
    pub fn into_normalized(mut self) -> Self {
        self.normalize();
        self
    }
}

impl_serde!(Origin<'a>, "an origin-form URI");

impl_traits!(Origin, path, query);

impl std::fmt::Display for Origin<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path())?;
        if let Some(query) = self.query() {
            write!(f, "?{}", query)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Origin;

    fn seg_count(path: &str, expected: usize) -> bool {
        let origin = Origin::parse(path).unwrap();
        let segments = origin.path().segments();
        let actual = segments.len();
        if actual != expected {
            eprintln!("Count mismatch: expected {}, got {}.", expected, actual);
            eprintln!("{}", if actual != expected { "lifetime" } else { "buf" });
            eprintln!("Segments (for {}):", path);
            for (i, segment) in segments.enumerate() {
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

        let actual: Vec<&str> = uri.path().segments().collect();
        actual == expected
    }

    #[test]
    fn send_and_sync() {
        fn assert<T: Send + Sync>() {}
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
        assert_eq!(uri.query().map(|q| q.as_str()), query);
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
            .into_normalized()
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
