use std::borrow::Cow;

use crate::ext::IntoOwned;
use crate::parse::{Extent, IndexedStr};
use crate::uri::{Authority, Path, Query, Data, Error, as_utf8_unchecked, fmt};

/// A URI with a scheme, authority, path, and query.
///
/// # Structure
///
/// The following diagram illustrates the syntactic structure of an absolute
/// URI with all optional parts:
///
/// ```text
///  http://user:pass@domain.com:4444/foo/bar?some=query
///  |--|  |------------------------||------| |--------|
/// scheme          authority          path      query
/// ```
///
/// Only the scheme part of the URI is required.
///
/// # Normalization
///
/// Rocket prefers _normalized_ absolute URIs, an absolute URI with the
/// following properties:
///
///   * The path and query, if any, are normalized with no empty segments.
///   * If there is an authority, the path is empty or absolute with more than
///     one character.
///
/// The [`Absolute::is_normalized()`] method checks for normalization while
/// [`Absolute::into_normalized()`] normalizes any absolute URI.
///
/// As an example, the following URIs are all valid, normalized URIs:
///
/// ```rust
/// # extern crate rocket;
/// # use rocket::http::uri::Absolute;
/// # let valid_uris = [
/// "http://rocket.rs",
/// "scheme:/foo/bar",
/// "scheme:/foo/bar?abc",
/// # ];
/// # for uri in &valid_uris {
/// #     let uri = Absolute::parse(uri).unwrap();
/// #     assert!(uri.is_normalized(), "{} non-normal?", uri);
/// # }
/// ```
///
/// By contrast, the following are valid but non-normal URIs:
///
/// ```rust
/// # extern crate rocket;
/// # use rocket::http::uri::Absolute;
/// # let invalid = [
/// "http://rocket.rs/",    // trailing '/'
/// "ftp:/a/b/",            // trailing empty segment
/// "ftp:/a//c//d",         // two empty segments
/// "ftp:/a/b/?",           // empty path segment
/// "ftp:/?foo&",           // trailing empty query segment
/// # ];
/// # for uri in &invalid {
/// #   assert!(!Absolute::parse(uri).unwrap().is_normalized());
/// # }
/// ```
///
/// # (De)serialization
///
/// `Absolute` is both `Serialize` and `Deserialize`:
///
/// ```rust
/// # #[cfg(feature = "serde")] mod serde {
/// # use serde_ as serde;
/// use serde::{Serialize, Deserialize};
/// use rocket::http::uri::Absolute;
///
/// #[derive(Deserialize, Serialize)]
/// # #[serde(crate = "serde_")]
/// struct UriOwned {
///     uri: Absolute<'static>,
/// }
///
/// #[derive(Deserialize, Serialize)]
/// # #[serde(crate = "serde_")]
/// struct UriBorrowed<'a> {
///     uri: Absolute<'a>,
/// }
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Absolute<'a> {
    pub(crate) source: Option<Cow<'a, str>>,
    pub(crate) scheme: IndexedStr<'a>,
    pub(crate) authority: Option<Authority<'a>>,
    pub(crate) path: Data<'a, fmt::Path>,
    pub(crate) query: Option<Data<'a, fmt::Query>>,
}

impl<'a> Absolute<'a> {
    /// Parses the string `string` into an `Absolute`. Parsing will never
    /// allocate. Returns an `Error` if `string` is not a valid absolute URI.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::uri::Absolute;
    ///
    /// // Parse a valid authority URI.
    /// let uri = Absolute::parse("https://rocket.rs").expect("valid URI");
    /// assert_eq!(uri.scheme(), "https");
    /// assert_eq!(uri.authority().unwrap().host(), "rocket.rs");
    /// assert_eq!(uri.path(), "");
    /// assert!(uri.query().is_none());
    ///
    /// // Prefer to use `uri!()` when the input is statically known:
    /// let uri = uri!("https://rocket.rs");
    /// assert_eq!(uri.scheme(), "https");
    /// assert_eq!(uri.authority().unwrap().host(), "rocket.rs");
    /// assert_eq!(uri.path(), "");
    /// assert!(uri.query().is_none());
    /// ```
    pub fn parse(string: &'a str) -> Result<Absolute<'a>, Error<'a>> {
        crate::parse::uri::absolute_from_str(string)
    }

    /// Parses the string `string` into an `Absolute`. Allocates minimally on
    /// success and error.
    ///
    /// This method should be used instead of [`Absolute::parse()`] when the
    /// source URI is already a `String`. Returns an `Error` if `string` is not
    /// a valid absolute URI.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Absolute;
    ///
    /// let source = format!("https://rocket.rs/foo/{}/three", 2);
    /// let uri = Absolute::parse_owned(source).expect("valid URI");
    /// assert_eq!(uri.authority().unwrap().host(), "rocket.rs");
    /// assert_eq!(uri.path(), "/foo/2/three");
    /// assert!(uri.query().is_none());
    /// ```
    // TODO: Avoid all allocations.
    pub fn parse_owned(string: String) -> Result<Absolute<'static>, Error<'static>> {
        let absolute = Absolute::parse(&string).map_err(|e| e.into_owned())?;
        debug_assert!(absolute.source.is_some(), "Absolute parsed w/o source");

        let absolute = Absolute {
            scheme: absolute.scheme.into_owned(),
            authority: absolute.authority.into_owned(),
            query: absolute.query.into_owned(),
            path: absolute.path.into_owned(),
            source: Some(Cow::Owned(string)),
        };

        Ok(absolute)
    }

    /// Returns the scheme part of the absolute URI.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// let uri = uri!("ftp://127.0.0.1");
    /// assert_eq!(uri.scheme(), "ftp");
    /// ```
    #[inline(always)]
    pub fn scheme(&self) -> &str {
        self.scheme.from_cow_source(&self.source)
    }

    /// Returns the authority part of the absolute URI, if there is one.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// let uri = uri!("https://rocket.rs:80");
    /// assert_eq!(uri.scheme(), "https");
    /// let authority = uri.authority().unwrap();
    /// assert_eq!(authority.host(), "rocket.rs");
    /// assert_eq!(authority.port(), Some(80));
    ///
    /// let uri = uri!("file:/web/home");
    /// assert_eq!(uri.authority(), None);
    /// ```
    #[inline(always)]
    pub fn authority(&self) -> Option<&Authority<'a>> {
        self.authority.as_ref()
    }

    /// Returns the path part. May be empty.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// let uri = uri!("ftp://rocket.rs/foo/bar");
    /// assert_eq!(uri.path(), "/foo/bar");
    ///
    /// let uri = uri!("ftp://rocket.rs");
    /// assert!(uri.path().is_empty());
    /// ```
    #[inline(always)]
    pub fn path(&self) -> Path<'_> {
        Path { source: &self.source, data: &self.path }
    }

    /// Returns the query part with the leading `?`. May be empty.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// let uri = uri!("ftp://rocket.rs/foo?bar");
    /// assert_eq!(uri.query().unwrap(), "bar");
    ///
    /// let uri = uri!("ftp://rocket.rs");
    /// assert!(uri.query().is_none());
    /// ```
    #[inline(always)]
    pub fn query(&self) -> Option<Query<'_>> {
        self.query.as_ref().map(|data| Query { source: &self.source, data })
    }

    /// Removes the query part of this URI, if there is any.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// let mut uri = uri!("ftp://rocket.rs/foo?bar");
    /// assert_eq!(uri.query().unwrap(), "bar");
    ///
    /// uri.clear_query();
    /// assert!(uri.query().is_none());
    /// ```
    #[inline(always)]
    pub fn clear_query(&mut self) {
        self.set_query(None);
    }

    /// Returns `true` if `self` is normalized. Otherwise, returns `false`.
    ///
    /// See [Normalization](#normalization) for more information on what it
    /// means for an absolute URI to be normalized. Note that `uri!()` always
    /// returns a normalized version of its static input.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::uri::Absolute;
    ///
    /// assert!(uri!("http://rocket.rs").is_normalized());
    /// assert!(uri!("http://rocket.rs///foo////bar").is_normalized());
    ///
    /// assert!(Absolute::parse("http:/").unwrap().is_normalized());
    /// assert!(Absolute::parse("http://").unwrap().is_normalized());
    /// assert!(Absolute::parse("http://foo.rs/foo/bar").unwrap().is_normalized());
    /// assert!(Absolute::parse("foo:bar").unwrap().is_normalized());
    ///
    /// assert!(!Absolute::parse("git://rocket.rs/").unwrap().is_normalized());
    /// assert!(!Absolute::parse("http:/foo//bar").unwrap().is_normalized());
    /// assert!(!Absolute::parse("foo:bar?baz&&bop").unwrap().is_normalized());
    /// ```
    pub fn is_normalized(&self) -> bool {
        let normalized_query = self.query().map_or(true, |q| q.is_normalized());
        if self.authority().is_some() && !self.path().is_empty() {
            self.path().is_normalized(true)
                && self.path() != "/"
                && normalized_query
        } else {
            self.path().is_normalized(false) && normalized_query
        }
    }

    /// Normalizes `self` in-place. Does nothing if `self` is already
    /// normalized.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::http::uri::Absolute;
    ///
    /// let mut uri = Absolute::parse("git://rocket.rs/").unwrap();
    /// assert!(!uri.is_normalized());
    /// uri.normalize();
    /// assert!(uri.is_normalized());
    ///
    /// let mut uri = Absolute::parse("http:/foo//bar").unwrap();
    /// assert!(!uri.is_normalized());
    /// uri.normalize();
    /// assert!(uri.is_normalized());
    ///
    /// let mut uri = Absolute::parse("foo:bar?baz&&bop").unwrap();
    /// assert!(!uri.is_normalized());
    /// uri.normalize();
    /// assert!(uri.is_normalized());
    /// ```
    pub fn normalize(&mut self) {
        if self.authority().is_some() && !self.path().is_empty() {
            if self.path() == "/" {
                self.set_path("");
            } else if !self.path().is_normalized(true) {
                self.path = self.path().to_normalized(true);
            }
        } else {
            self.path = self.path().to_normalized(false);
        }

        if let Some(query) = self.query() {
            if !query.is_normalized() {
                self.query = query.to_normalized();
            }
        }
    }

    /// Normalizes `self`. This is a no-op if `self` is already normalized.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::http::uri::Absolute;
    ///
    /// let mut uri = Absolute::parse("git://rocket.rs/").unwrap();
    /// assert!(!uri.is_normalized());
    /// assert!(uri.into_normalized().is_normalized());
    ///
    /// let mut uri = Absolute::parse("http:/foo//bar").unwrap();
    /// assert!(!uri.is_normalized());
    /// assert!(uri.into_normalized().is_normalized());
    ///
    /// let mut uri = Absolute::parse("foo:bar?baz&&bop").unwrap();
    /// assert!(!uri.is_normalized());
    /// assert!(uri.into_normalized().is_normalized());
    /// ```
    pub fn into_normalized(mut self) -> Self {
        self.normalize();
        self
    }

    /// Sets the authority in `self` to `authority`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// let mut uri = uri!("https://rocket.rs:80");
    /// let authority = uri.authority().unwrap();
    /// assert_eq!(authority.host(), "rocket.rs");
    /// assert_eq!(authority.port(), Some(80));
    ///
    /// let new_authority = uri!("rocket.rs:443");
    /// uri.set_authority(new_authority);
    /// let authority = uri.authority().unwrap();
    /// assert_eq!(authority.host(), "rocket.rs");
    /// assert_eq!(authority.port(), Some(443));
    /// ```
    #[inline(always)]
    pub fn set_authority(&mut self, authority: Authority<'a>) {
        self.authority = Some(authority);
    }

    /// Sets the authority in `self` to `authority` and returns `self`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// let uri = uri!("https://rocket.rs:80");
    /// let authority = uri.authority().unwrap();
    /// assert_eq!(authority.host(), "rocket.rs");
    /// assert_eq!(authority.port(), Some(80));
    ///
    /// let new_authority = uri!("rocket.rs");
    /// let uri = uri.with_authority(new_authority);
    /// let authority = uri.authority().unwrap();
    /// assert_eq!(authority.host(), "rocket.rs");
    /// assert_eq!(authority.port(), None);
    /// ```
    #[inline(always)]
    pub fn with_authority(mut self, authority: Authority<'a>) -> Self {
        self.set_authority(authority);
        self
    }
}

/// PRIVATE API.
#[doc(hidden)]
impl<'a> Absolute<'a> {
    /// PRIVATE. Used by parser.
    ///
    /// SAFETY: `source` must be valid UTF-8.
    /// CORRECTNESS: `scheme` must be non-empty.
    #[inline]
    pub(crate) unsafe fn raw(
        source: Cow<'a, [u8]>,
        scheme: Extent<&'a [u8]>,
        authority: Option<Authority<'a>>,
        path: Extent<&'a [u8]>,
        query: Option<Extent<&'a [u8]>>,
    ) -> Absolute<'a> {
        Absolute {
            source: Some(as_utf8_unchecked(source)),
            scheme: scheme.into(),
            authority,
            path: Data::raw(path),
            query: query.map(Data::raw)
        }
    }

    /// PRIVATE. Used by tests.
    #[cfg(test)]
    pub fn new(
        scheme: &'a str,
        authority: impl Into<Option<Authority<'a>>>,
        path: &'a str,
        query: impl Into<Option<&'a str>>,
    ) -> Absolute<'a> {
        assert!(!scheme.is_empty());
        Absolute::const_new(scheme, authority.into(), path, query.into())
    }

    /// PRIVATE. Used by codegen and `Host`.
    #[doc(hidden)]
    pub const fn const_new(
        scheme: &'a str,
        authority: Option<Authority<'a>>,
        path: &'a str,
        query: Option<&'a str>,
    ) -> Absolute<'a> {
        Absolute {
            source: None,
            scheme: IndexedStr::Concrete(Cow::Borrowed(scheme)),
            authority,
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

    // TODO: Have a way to get a validated `path` to do this. See `Path`?
    pub(crate) fn set_path<P>(&mut self, path: P)
        where P: Into<Cow<'a, str>>
    {
        self.path = Data::new(path.into());
    }

    // TODO: Have a way to get a validated `query` to do this. See `Query`?
    pub(crate) fn set_query<Q: Into<Option<Cow<'a, str>>>>(&mut self, query: Q) {
        self.query = query.into().map(Data::new);
    }
}

impl_serde!(Absolute<'a>, "an absolute-form URI");

impl_traits!(Absolute, scheme, authority, path, query);

impl std::fmt::Display for Absolute<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:", self.scheme())?;
        if let Some(authority) = self.authority() {
            write!(f, "//{}", authority)?;
        }

        write!(f, "{}", self.path())?;
        if let Some(query) = self.query() {
            write!(f, "?{}", query)?;
        }

        Ok(())
    }
}
