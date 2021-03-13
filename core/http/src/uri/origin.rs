use std::borrow::Cow;
use std::convert::TryFrom;
use std::fmt::{self, Display};
use std::hash::Hash;

use crate::ext::IntoOwned;
use crate::parse::{Indexed, Extent, IndexedStr, uri::tables::is_pchar};
use crate::uri::{self, UriPart, Query, Path};
use crate::uri::{Error, Segments, QuerySegments, as_utf8_unchecked};
use crate::{RawStr, RawStrBuf};

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
/// "/hello?lang=en",
/// "/some%20thing?q=foo&lang=fr",
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
/// // abnormal versions
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
#[derive(Clone, Debug)]
pub struct Origin<'a> {
    pub(crate) source: Option<Cow<'a, str>>,
    pub(crate) path: IndexedStr<'a>,
    pub(crate) query: Option<IndexedStr<'a>>,

    pub(crate) decoded_path_segs: Storage<Vec<IndexedStr<'static>>>,
    pub(crate) decoded_query_segs: Storage<Vec<(IndexedStr<'static>, IndexedStr<'static>)>>,
}

impl Hash for Origin<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.path().hash(state);
        self.query().hash(state);
    }
}

impl<'a, 'b> PartialEq<Origin<'b>> for Origin<'a> {
    fn eq(&self, other: &Origin<'b>) -> bool {
        self.path() == other.path() && self.query() == other.query()
    }
}

impl Eq for Origin<'_> { }

impl PartialEq<str> for Origin<'_> {
    fn eq(&self, other: &str) -> bool {
        let (path, query) = RawStr::new(other).split_at_byte(b'?');
        self.path() == path && self.query().unwrap_or("".into()) == query
    }
}

impl PartialEq<&str> for Origin<'_> {
    fn eq(&self, other: &&str) -> bool {
        self.eq(*other)
    }
}

impl PartialEq<Origin<'_>> for str {
    fn eq(&self, other: &Origin<'_>) -> bool {
        other.eq(self)
    }
}

impl IntoOwned for Origin<'_> {
    type Owned = Origin<'static>;

    fn into_owned(self) -> Origin<'static> {
        Origin {
            source: self.source.into_owned(),
            path: self.path.into_owned(),
            query: self.query.into_owned(),
            decoded_path_segs: self.decoded_path_segs.map(|v| v.into_owned()),
            decoded_query_segs: self.decoded_query_segs.map(|v| v.into_owned()),
        }
    }
}

fn decode_to_indexed_str<P: UriPart>(
    value: &RawStr,
    (indexed, source): (&IndexedStr<'_>, &RawStr)
) -> IndexedStr<'static> {
    let decoded = match P::KIND {
        uri::Kind::Path => value.percent_decode_lossy(),
        uri::Kind::Query => value.url_decode_lossy(),
    };

    match decoded {
        Cow::Borrowed(b) if indexed.is_indexed() => {
            let indexed = IndexedStr::checked_from(b, source.as_str());
            debug_assert!(indexed.is_some());
            indexed.unwrap_or_else(|| IndexedStr::from(Cow::Borrowed("")))
        }
        cow => IndexedStr::from(Cow::Owned(cow.into_owned())),
    }
}

impl<'a> Origin<'a> {
    /// SAFETY: `source` must be UTF-8.
    #[inline]
    pub(crate) unsafe fn raw(
        source: Cow<'a, [u8]>,
        path: Extent<&'a [u8]>,
        query: Option<Extent<&'a [u8]>>
    ) -> Origin<'a> {
        Origin {
            source: Some(as_utf8_unchecked(source)),
            path: path.into(),
            query: query.map(|q| q.into()),

            decoded_path_segs: Storage::new(),
            decoded_query_segs: Storage::new(),
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
            path: Indexed::from(path.into()),
            query: query.map(|q| Indexed::from(q.into())),
            decoded_path_segs: Storage::new(),
            decoded_query_segs: Storage::new(),
        }
    }

    // Used to fabricate URIs in several places. Equivalent to `Origin::new("/",
    // None)` or `Origin::parse("/").unwrap()`. Should not be used outside of
    // Rocket, though doing so would be harmless.
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
    /// assert_eq!(uri.query().unwrap(), "query");
    ///
    /// // Invalid URIs fail to parse.
    /// Origin::parse("foo bar").expect_err("invalid URI");
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
        let query = match query.is_empty() {
            false => Some(query.as_str()),
            true => None,
        };

        Ok(Origin::new(path.as_str(), query))
    }

    /// Parses the string `string` into an `Origin`. Parsing will never
    /// allocate. This method should be used instead of
    /// [`Origin::parse()`](crate::uri::Origin::parse()) when the source URI is
    /// already a `String`. Returns an `Error` if `string` is not a valid origin
    /// URI.
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
        //  2. `string` isn't dropped while `copy_of_str` is live.
        //
        // These two facts can be easily verified. An `&mut` can't be created
        // because `string` isn't `mut`. Then, `string` is clearly not dropped
        // since it's passed in to `source`.
        // let copy_of_str = unsafe { &*(string.as_str() as *const str) };
        let copy_of_str = unsafe { &*(string.as_str() as *const str) };
        let origin = Origin::parse(copy_of_str)?;
        debug_assert!(origin.source.is_some(), "Origin source parsed w/o source");

        let origin = Origin {
            path: origin.path.into_owned(),
            query: origin.query.into_owned(),
            decoded_path_segs: origin.decoded_path_segs.into_owned(),
            decoded_query_segs: origin.decoded_query_segs.into_owned(),
            // At this point, it's impossible for anything to be borrowing
            // `string` except for `source`, even though Rust doesn't know it.
            // Because we're replacing `source` here, there can't possibly be a
            // borrow remaining, it's safe to "move out of the borrow".
            source: Some(Cow::Owned(string)),
        };

        Ok(origin)
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
    /// let normal = Origin::parse("/a/b/c?a=b&c").unwrap();
    /// assert!(normal.is_normalized());
    ///
    /// let abnormal = Origin::parse("/a/b/c//d").unwrap();
    /// assert!(!abnormal.is_normalized());
    ///
    /// let abnormal = Origin::parse("/a?q&&b").unwrap();
    /// assert!(!abnormal.is_normalized());
    /// ```
    pub fn is_normalized(&self) -> bool {
        self.path().starts_with('/')
            && self.raw_path_segments().all(|s| !s.is_empty())
            && self.raw_query_segments().all(|s| !s.is_empty())
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
    /// let normalized = abnormal.into_normalized();
    /// assert!(normalized.is_normalized());
    /// assert_eq!(normalized, Origin::parse("/a/b/c/d").unwrap());
    /// ```
    pub fn into_normalized(mut self) -> Self {
        use std::fmt::Write;

        if self.is_normalized() {
            self
        } else {
            let mut new_path = String::with_capacity(self.path().len());
            for seg in self.raw_path_segments().filter(|s| !s.is_empty()) {
                let _ = write!(new_path, "/{}", seg);
            }

            if new_path.is_empty() {
                new_path.push('/');
            }

            self.path = Indexed::from(Cow::Owned(new_path));

            if let Some(q) = self.query() {
                let mut new_query = String::with_capacity(q.len());
                let raw_segments = self.raw_query_segments()
                    .filter(|s| !s.is_empty())
                    .enumerate();

                for (i, seg) in raw_segments {
                    if i != 0 { new_query.push('&'); }
                    let _ = write!(new_query, "{}", seg);
                }

                self.query = Some(Indexed::from(Cow::Owned(new_query)));
            }

            // Note: normalization preserves segments!
            self
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
    pub fn path(&self) -> &RawStr {
        self.path.from_cow_source(&self.source).into()
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
    /// # extern crate rocket;
    /// use rocket::http::uri::Origin;
    ///
    /// let old_uri = Origin::parse("/a/b/c").unwrap();
    /// let expected_uri = Origin::parse("/a/b/c/").unwrap();
    /// assert_eq!(old_uri.map_path(|p| format!("{}/", p)), Some(expected_uri));
    ///
    /// let old_uri = Origin::parse("/a/b/c/").unwrap();
    /// let expected_uri = Origin::parse("/a/b/c//").unwrap();
    /// assert_eq!(old_uri.map_path(|p| format!("{}/", p)), Some(expected_uri));
    ///
    /// let old_uri = Origin::parse("/a/b/c/").unwrap();
    /// let expected = Origin::parse("/b/c/").unwrap();
    /// assert_eq!(old_uri.map_path(|p| p.strip_prefix("/a").unwrap_or(p)), Some(expected));
    ///
    /// let old_uri = Origin::parse("/a").unwrap();
    /// assert_eq!(old_uri.map_path(|p| p.strip_prefix("/a").unwrap_or(p)), None);
    ///
    /// let old_uri = Origin::parse("/a/b/c/").unwrap();
    /// assert_eq!(old_uri.map_path(|p| format!("hi/{}", p)), None);
    /// ```
    #[inline]
    pub fn map_path<'s, F, P>(&'s self, f: F) -> Option<Self>
        where F: FnOnce(&'s RawStr) -> P, P: Into<RawStrBuf> + 's
    {
        let path = f(self.path()).into();
        if !path.starts_with('/') || !path.as_bytes().iter().all(|b| is_pchar(&b)) {
            return None;
        }

        Some(Origin {
            source: self.source.clone(),
            path: Cow::from(path.into_string()).into(),
            query: self.query.clone(),
            decoded_path_segs: Storage::new(),
            decoded_query_segs: Storage::new(),
        })
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
    /// assert_eq!(uri.query().unwrap(), "alphabet=true");
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
    pub fn query(&self) -> Option<&RawStr> {
        self.query.as_ref().map(|q| q.from_cow_source(&self.source).into())
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
    /// assert_eq!(uri.query().unwrap(), "query=some");
    ///
    /// uri.clear_query();
    /// assert_eq!(uri.query(), None);
    /// ```
    pub fn clear_query(&mut self) {
        self.query = None;
    }

    /// Returns a (smart) iterator over the non-empty, percent-decoded segments
    /// of the path of this URI.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Origin;
    ///
    /// let uri = Origin::parse("/a%20b/b%2Fc/d//e?query=some").unwrap();
    /// let path_segs: Vec<&str> = uri.path_segments().collect();
    /// assert_eq!(path_segs, &["a b", "b/c", "d", "e"]);
    /// ```
    pub fn path_segments(&self) -> Segments<'_> {
        let cached = self.decoded_path_segs.get_or_set(|| {
            let (indexed, path) = (&self.path, self.path());
            self.raw_path_segments()
                .filter(|r| !r.is_empty())
                .map(|s| decode_to_indexed_str::<Path>(s, (indexed, path)))
                .collect()
        });

        Segments { source: self.path(), segments: cached, pos: 0 }
    }

    /// Returns a (smart) iterator over the non-empty, url-decoded `(name,
    /// value)` pairs of the query of this URI. If there is no query, the
    /// iterator is empty.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Origin;
    ///
    /// let uri = Origin::parse("/").unwrap();
    /// let query_segs: Vec<_> = uri.query_segments().collect();
    /// assert!(query_segs.is_empty());
    ///
    /// let uri = Origin::parse("/foo/bar?a+b%2F=some+one%40gmail.com&&%26%3D2").unwrap();
    /// let query_segs: Vec<_> = uri.query_segments().collect();
    /// assert_eq!(query_segs, &[("a b/", "some one@gmail.com"), ("&=2", "")]);
    /// ```
    pub fn query_segments(&self) -> QuerySegments<'_> {
        let cached = self.decoded_query_segs.get_or_set(|| {
            let (indexed, query) = match (self.query.as_ref(), self.query()) {
                (Some(i), Some(q)) => (i, q),
                _ => return vec![],
            };

            self.raw_query_segments()
                .filter(|s| !s.is_empty())
                .map(|s| s.split_at_byte(b'='))
                .map(|(name, val)| {
                    let name = decode_to_indexed_str::<Query>(name, (indexed, query));
                    let val = decode_to_indexed_str::<Query>(val, (indexed, query));
                    (name, val)
                })
                .collect()
        });

        QuerySegments { source: self.query(), segments: cached, pos: 0 }
    }

    /// Returns an iterator over the raw, undecoded segments of the path in this
    /// URI. Segments may be empty.
    ///
    /// ### Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Origin;
    ///
    /// let uri = Origin::parse("/").unwrap();
    /// let segments: Vec<_> = uri.raw_path_segments().collect();
    /// assert!(segments.is_empty());
    ///
    /// let uri = Origin::parse("//").unwrap();
    /// let segments: Vec<_> = uri.raw_path_segments().collect();
    /// assert_eq!(segments, &["", ""]);
    ///
    /// let uri = Origin::parse("/a").unwrap();
    /// let segments: Vec<_> = uri.raw_path_segments().collect();
    /// assert_eq!(segments, &["a"]);
    ///
    /// let uri = Origin::parse("/a//b///c/d?query&param").unwrap();
    /// let segments: Vec<_> = uri.raw_path_segments().collect();
    /// assert_eq!(segments, &["a", "", "b", "", "", "c", "d"]);
    /// ```
    #[inline(always)]
    pub fn raw_path_segments(&self) -> impl Iterator<Item = &RawStr> {
        let path = match self.path() {
            p if p == "/" => None,
            p if p.starts_with('/') => Some(&p[1..]),
            p => Some(p)
        };

        path.map(|p| p.split(Path::DELIMITER))
            .into_iter()
            .flatten()
    }

    /// Returns an iterator over the non-empty, non-url-decoded `(name, value)`
    /// pairs of the query of this URI. If there is no query, the iterator is
    /// empty. Segments may be empty.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Origin;
    ///
    /// let uri = Origin::parse("/").unwrap();
    /// assert!(uri.raw_query_segments().next().is_none());
    ///
    /// let uri = Origin::parse("/?a=b&dog").unwrap();
    /// let query_segs: Vec<_> = uri.raw_query_segments().collect();
    /// assert_eq!(query_segs, &["a=b", "dog"]);
    ///
    /// let uri = Origin::parse("/?&").unwrap();
    /// let query_segs: Vec<_> = uri.raw_query_segments().collect();
    /// assert_eq!(query_segs, &["", ""]);
    ///
    /// let uri = Origin::parse("/foo/bar?a+b%2F=some+one%40gmail.com&&%26%3D2").unwrap();
    /// let query_segs: Vec<_> = uri.raw_query_segments().collect();
    /// assert_eq!(query_segs, &["a+b%2F=some+one%40gmail.com", "", "%26%3D2"]);
    /// ```
    #[inline]
    pub fn raw_query_segments(&self) -> impl Iterator<Item = &RawStr> {
        self.query()
            .into_iter()
            .flat_map(|q| q.split(Query::DELIMITER))
    }
}

impl TryFrom<String> for Origin<'static> {
    type Error = Error<'static>;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Origin::parse_owned(value)
    }
}

// Because inference doesn't take `&String` to `&str`.
impl<'a> TryFrom<&'a String> for Origin<'a> {
    type Error = Error<'a>;

    fn try_from(value: &'a String) -> Result<Self, Self::Error> {
        Origin::parse(value.as_str())
    }
}

impl<'a> TryFrom<&'a str> for Origin<'a> {
    type Error = Error<'a>;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        Origin::parse(value)
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
        let origin = Origin::parse(path).unwrap();
        let segments = origin.path_segments();
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

        let actual: Vec<&str> = uri.path_segments().collect();
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
        assert_eq!(uri.query().map(|s| s.as_str()), query);
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
