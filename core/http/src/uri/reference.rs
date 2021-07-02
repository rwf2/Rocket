use std::borrow::Cow;

use crate::RawStr;
use crate::ext::IntoOwned;
use crate::uri::{Authority, Data, Origin, Absolute, Asterisk};
use crate::uri::{Path, Query, Error, as_utf8_unchecked, fmt};
use crate::parse::{Extent, IndexedStr};

/// A URI-reference with optional scheme, authority, relative path, query, and
/// fragment parts.
///
/// # Structure
///
/// The following diagram illustrates the syntactic structure of a URI reference
/// with all optional parts:
///
/// ```text
///  http://user:pass@domain.com:4444/foo/bar?some=query#and-fragment
///  |--|  |------------------------||------| |--------| |----------|
/// scheme          authority          path      query     fragment
/// ```
///
/// All parts are optional. When a scheme and authority are not present, the
/// path may be relative: `foo/bar?baz#cat`.
///
/// # Conversion
///
/// All other URI types ([`Origin`], [`Absolute`], and so on) are valid URI
/// references. As such, conversion between the types is lossless:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// use rocket::http::uri::Reference;
///
/// let absolute = uri!("http://rocket.rs");
/// let reference: Reference = absolute.into();
/// assert_eq!(reference.scheme(), Some("http"));
/// assert_eq!(reference.authority().unwrap().host(), "rocket.rs");
///
/// let origin = uri!("/foo/bar");
/// let reference: Reference = origin.into();
/// assert_eq!(reference.path(), "/foo/bar");
/// ```
///
/// Note that `uri!()` macro _always_ prefers the more specific URI variant to
/// `Reference` when possible, as is demonstrated above for `absolute` and
/// `origin`.
///
/// # (De)serialization
///
/// `Reference` is both `Serialize` and `Deserialize`:
///
/// ```rust
/// # #[cfg(feature = "serde")] mod serde {
/// # use serde_ as serde;
/// use serde::{Serialize, Deserialize};
/// use rocket::http::uri::Reference;
///
/// #[derive(Deserialize, Serialize)]
/// # #[serde(crate = "serde_")]
/// struct UriOwned {
///     uri: Reference<'static>,
/// }
///
/// #[derive(Deserialize, Serialize)]
/// # #[serde(crate = "serde_")]
/// struct UriBorrowed<'a> {
///     uri: Reference<'a>,
/// }
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Reference<'a> {
    source: Option<Cow<'a, str>>,
    scheme: Option<IndexedStr<'a>>,
    authority: Option<Authority<'a>>,
    path: Data<'a, fmt::Path>,
    query: Option<Data<'a, fmt::Query>>,
    fragment: Option<IndexedStr<'a>>,
}

impl<'a> Reference<'a> {
    #[inline]
    pub(crate) unsafe fn raw(
        source: Cow<'a, [u8]>,
        scheme: Option<Extent<&'a [u8]>>,
        authority: Option<Authority<'a>>,
        path: Extent<&'a [u8]>,
        query: Option<Extent<&'a [u8]>>,
        fragment: Option<Extent<&'a [u8]>>,
    ) -> Reference<'a> {
        Reference {
            source: Some(as_utf8_unchecked(source)),
            scheme: scheme.map(|s| s.into()),
            authority,
            path: Data::raw(path),
            query: query.map(Data::raw),
            fragment: fragment.map(|f| f.into()),
        }
    }

    /// PRIVATE. Used during test.
    #[cfg(test)]
    pub fn new(
        scheme: impl Into<Option<&'a str>>,
        auth: impl Into<Option<Authority<'a>>>,
        path: &'a str,
        query: impl Into<Option<&'a str>>,
        frag: impl Into<Option<&'a str>>,
    ) -> Reference<'a> {
        Reference::const_new(scheme.into(), auth.into(), path, query.into(), frag.into())
    }

    /// PRIVATE. Used by codegen.
    #[doc(hidden)]
    pub const fn const_new(
        scheme: Option<&'a str>,
        authority: Option<Authority<'a>>,
        path: &'a str,
        query: Option<&'a str>,
        fragment: Option<&'a str>,
    ) -> Reference<'a> {
        Reference {
            source: None,
            scheme: match scheme {
                Some(scheme) => Some(IndexedStr::Concrete(Cow::Borrowed(scheme))),
                None => None
            },
            authority,
            path: Data {
                value: IndexedStr::Concrete(Cow::Borrowed(path)),
                decoded_segments: state::Storage::new(),
            },
            query: match query {
                Some(query) => Some(Data {
                    value: IndexedStr::Concrete(Cow::Borrowed(query)),
                    decoded_segments: state::Storage::new(),
                }),
                None => None,
            },
            fragment: match fragment {
                Some(frag) => Some(IndexedStr::Concrete(Cow::Borrowed(frag))),
                None => None,
            },
        }
    }

    /// Parses the string `string` into an `Reference`. Parsing will never
    /// allocate. Returns an `Error` if `string` is not a valid origin URI.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::uri::Reference;
    ///
    /// // Parse a valid URI reference.
    /// let uri = Reference::parse("/a/b/c?query").expect("valid URI");
    /// assert_eq!(uri.path(), "/a/b/c");
    /// assert_eq!(uri.query().unwrap(), "query");
    ///
    /// // Invalid URIs fail to parse.
    /// Reference::parse("foo bar").expect_err("invalid URI");
    ///
    /// // Prefer to use `uri!()` when the input is statically known:
    /// let uri = uri!("/a/b/c?query#fragment");
    /// assert_eq!(uri.path(), "/a/b/c");
    /// assert_eq!(uri.query().unwrap(), "query");
    /// assert_eq!(uri.fragment().unwrap(), "fragment");
    /// ```
    pub fn parse(string: &'a str) -> Result<Reference<'a>, Error<'a>> {
        crate::parse::uri::reference_from_str(string)
    }

    /// Parses the string `string` into a `Reference`. Allocates minimally on
    /// success and error.
    ///
    /// This method should be used instead of [`Reference::parse()`] when the
    /// source URI is already a `String`. Returns an `Error` if `string` is not
    /// a valid URI reference.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Reference;
    ///
    /// let source = format!("/foo?{}#3", 2);
    /// let uri = Reference::parse_owned(source).unwrap();
    /// assert_eq!(uri.path(), "/foo");
    /// assert_eq!(uri.query().unwrap(), "2");
    /// assert_eq!(uri.fragment().unwrap(), "3");
    /// ```
    // TODO: Avoid all allocations.
    pub fn parse_owned(string: String) -> Result<Reference<'static>, Error<'static>> {
        let uri_ref = Reference::parse(&string).map_err(|e| e.into_owned())?;
        debug_assert!(uri_ref.source.is_some(), "Reference parsed w/o source");

        Ok(Reference {
            scheme: uri_ref.scheme.into_owned(),
            authority: uri_ref.authority.into_owned(),
            path: uri_ref.path.into_owned(),
            query: uri_ref.query.into_owned(),
            fragment: uri_ref.fragment.into_owned(),
            source: Some(Cow::Owned(string)),
        })
    }

    /// Returns the scheme. If `Some`, is non-empty.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// let uri = uri!("http://rocket.rs?foo#bar");
    /// assert_eq!(uri.scheme(), Some("http"));
    ///
    /// let uri = uri!("ftp:/?foo#bar");
    /// assert_eq!(uri.scheme(), Some("ftp"));
    ///
    /// let uri = uri!("?foo#bar");
    /// assert_eq!(uri.scheme(), None);
    /// ```
    #[inline]
    pub fn scheme(&self) -> Option<&str> {
        self.scheme.as_ref().map(|s| s.from_cow_source(&self.source))
    }

    /// Returns the authority part.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// let uri = uri!("http://rocket.rs:4444?foo#bar");
    /// let auth = uri!("rocket.rs:4444");
    /// assert_eq!(uri.authority().unwrap(), &auth);
    ///
    /// let uri = uri!("?foo#bar");
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
    /// let uri = uri!("http://rocket.rs/guide?foo#bar");
    /// assert_eq!(uri.path(), "/guide");
    /// ```
    #[inline(always)]
    pub fn path(&self) -> Path<'_> {
        Path { source: &self.source, data: &self.path }
    }

    /// Returns the query part. May be empty.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// let uri = uri!("http://rocket.rs/guide?foo#bar");
    /// assert_eq!(uri.query().unwrap(), "foo");
    ///
    /// let uri = uri!("http://rocket.rs/guide?q=bar");
    /// assert_eq!(uri.query().unwrap(), "q=bar");
    ///
    /// // Empty query parts are normalized away by `uri!()`.
    /// let uri = uri!("http://rocket.rs/guide?#bar");
    /// assert!(uri.query().is_none());
    /// ```
    #[inline(always)]
    pub fn query(&self) -> Option<Query<'_>> {
        self.query.as_ref().map(|data| Query { source: &self.source, data })
    }

    /// Returns the fragment part, if any.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// let uri = uri!("http://rocket.rs/guide?foo#bar");
    /// assert_eq!(uri.fragment().unwrap(), "bar");
    ///
    /// // Fragment parts aren't normalized away, unlike query parts.
    /// let uri = uri!("http://rocket.rs/guide?foo#");
    /// assert_eq!(uri.fragment().unwrap(), "");
    /// ```
    #[inline(always)]
    pub fn fragment(&self) -> Option<&RawStr> {
        self.fragment.as_ref()
            .map(|frag| frag.from_cow_source(&self.source).into())
    }

    /// Returns `true` if `self` is normalized. Otherwise, returns `false`.
    ///
    /// Normalization for a URI reference is equivalent to normalization for an
    /// absolute URI. See [`Absolute#normalization`] for more information on
    /// what it means for an absolute URI to be normalized.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::uri::Reference;
    ///
    /// assert!(Reference::parse("foo/bar").unwrap().is_normalized());
    /// assert!(Reference::parse("foo/bar#").unwrap().is_normalized());
    /// assert!(Reference::parse("http://").unwrap().is_normalized());
    /// assert!(Reference::parse("http://foo.rs/foo/bar").unwrap().is_normalized());
    /// assert!(Reference::parse("foo:bar#baz").unwrap().is_normalized());
    /// assert!(Reference::parse("http://rocket.rs#foo").unwrap().is_normalized());
    ///
    /// assert!(!Reference::parse("http://?").unwrap().is_normalized());
    /// assert!(!Reference::parse("git://rocket.rs/").unwrap().is_normalized());
    /// assert!(!Reference::parse("http:/foo//bar").unwrap().is_normalized());
    /// assert!(!Reference::parse("foo:bar?baz&&bop#c").unwrap().is_normalized());
    /// assert!(!Reference::parse("http://rocket.rs?#foo").unwrap().is_normalized());
    ///
    /// // Recall that `uri!()` normalizes static input.
    /// assert!(uri!("http://rocket.rs#foo").is_normalized());
    /// assert!(uri!("http://rocket.rs///foo////bar#cat").is_normalized());
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
    /// use rocket::http::uri::Reference;
    ///
    /// let mut uri = Reference::parse("git://rocket.rs/").unwrap();
    /// assert!(!uri.is_normalized());
    /// uri.normalize();
    /// assert!(uri.is_normalized());
    ///
    /// let mut uri = Reference::parse("http:/foo//bar?baz&&#cat").unwrap();
    /// assert!(!uri.is_normalized());
    /// uri.normalize();
    /// assert!(uri.is_normalized());
    ///
    /// let mut uri = Reference::parse("foo:bar?baz&&bop").unwrap();
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
    /// use rocket::http::uri::Reference;
    ///
    /// let mut uri = Reference::parse("git://rocket.rs/").unwrap();
    /// assert!(!uri.is_normalized());
    /// assert!(uri.into_normalized().is_normalized());
    ///
    /// let mut uri = Reference::parse("http:/foo//bar?baz&&#cat").unwrap();
    /// assert!(!uri.is_normalized());
    /// assert!(uri.into_normalized().is_normalized());
    ///
    /// let mut uri = Reference::parse("foo:bar?baz&&bop").unwrap();
    /// assert!(!uri.is_normalized());
    /// assert!(uri.into_normalized().is_normalized());
    /// ```
    pub fn into_normalized(mut self) -> Self {
        self.normalize();
        self
    }

    pub(crate) fn set_path<P>(&mut self, path: P)
        where P: Into<Cow<'a, str>>
    {
        self.path = Data::new(path.into());
    }

    /// Returns the conrete path and query.
    pub(crate) fn with_query_fragment_of(mut self, other: Reference<'a>) -> Self {
        if let Some(query) = other.query {
            if self.query().is_none() {
                self.query = Some(Data::new(query.value.into_concrete(&self.source)));
            }
        }

        if let Some(frag) = other.fragment {
            if self.fragment().is_none() {
                self.fragment = Some(IndexedStr::from(frag.into_concrete(&self.source)));
            }
        }

        self
    }
}

impl_traits!(Reference, authority, scheme, path, query, fragment);

impl_serde!(Reference<'a>, "a URI-reference");

impl<'a> From<Absolute<'a>> for Reference<'a> {
    fn from(absolute: Absolute<'a>) -> Self {
        Reference {
            source: absolute.source,
            scheme: Some(absolute.scheme),
            authority: absolute.authority,
            path: absolute.path,
            query: absolute.query,
            fragment: None,
        }
    }
}

impl<'a> From<Origin<'a>> for Reference<'a> {
    fn from(origin: Origin<'a>) -> Self {
        Reference {
            source: origin.source,
            scheme: None,
            authority: None,
            path: origin.path,
            query: origin.query,
            fragment: None,
        }
    }
}

impl<'a> From<Authority<'a>> for Reference<'a> {
    fn from(authority: Authority<'a>) -> Self {
        Reference {
            source: match authority.source {
                Some(Cow::Borrowed(b)) => Some(Cow::Borrowed(b)),
                _ => None
            },
            authority: Some(authority),
            scheme: None,
            path: Data::new(""),
            query: None,
            fragment: None,
        }
    }
}

impl From<Asterisk> for Reference<'_> {
    fn from(_: Asterisk) -> Self {
        Reference {
            source: None,
            authority: None,
            scheme: None,
            path: Data::new("*"),
            query: None,
            fragment: None,
        }
    }
}

impl std::fmt::Display for Reference<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(scheme) = self.scheme() {
            write!(f, "{}:", scheme)?;
        }

        if let Some(authority) = self.authority() {
            write!(f, "//{}", authority)?;
        }

        write!(f, "{}", self.path())?;

        if let Some(query) = self.query() {
            write!(f, "?{}", query)?;
        }

        if let Some(frag) = self.fragment() {
            write!(f, "#{}", frag)?;
        }

        Ok(())
    }
}
