use std::fmt::{self, Display};
use std::borrow::Cow;

use crate::ext::IntoOwned;
use crate::parse::{Extent, IndexedStr};
use crate::uri::{as_utf8_unchecked, error::Error};

/// A URI with an authority only: `user:pass@host:8000`.
///
/// # Structure
///
/// The following diagram illustrates the syntactic structure of an authority
/// URI:
///
/// ```text
/// username:password@some.host:8088
/// |---------------| |-------| |--|
///     user info        host   port
/// ```
///
/// Only the host part of the URI is required.
///
/// # (De)serialization
///
/// `Authority` is both `Serialize` and `Deserialize`:
///
/// ```rust
/// # #[cfg(feature = "serde")] mod serde {
/// # use serde_ as serde;
/// use serde::{Serialize, Deserialize};
/// use rocket::http::uri::Authority;
///
/// #[derive(Deserialize, Serialize)]
/// # #[serde(crate = "serde_")]
/// struct UriOwned {
///     uri: Authority<'static>,
/// }
///
/// #[derive(Deserialize, Serialize)]
/// # #[serde(crate = "serde_")]
/// struct UriBorrowed<'a> {
///     uri: Authority<'a>,
/// }
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Authority<'a> {
    pub(crate) source: Option<Cow<'a, str>>,
    pub(crate) user_info: Option<IndexedStr<'a>>,
    host: IndexedStr<'a>,
    port: Option<u16>,
}

impl<'a> Authority<'a> {
    // SAFETY: `source` must be valid UTF-8.
    // CORRECTNESS: `host` must be non-empty.
    pub(crate) unsafe fn raw(
        source: Cow<'a, [u8]>,
        user_info: Option<Extent<&'a [u8]>>,
        host: Extent<&'a [u8]>,
        port: Option<u16>
    ) -> Authority<'a> {
        Authority {
            source: Some(as_utf8_unchecked(source)),
            user_info: user_info.map(IndexedStr::from),
            host: IndexedStr::from(host),
            port,
        }
    }

    /// PRIVATE. Used by core.
    #[doc(hidden)]
    pub fn new(
        user_info: impl Into<Option<&'a str>>,
        host: &'a str,
        port: impl Into<Option<u16>>,
    ) -> Self {
        Authority::const_new(user_info.into(), host, port.into())
    }

    /// PRIVATE. Used by codegen.
    #[doc(hidden)]
    pub const fn const_new(user_info: Option<&'a str>, host: &'a str, port: Option<u16>) -> Self {
        Authority {
            source: None,
            user_info: match user_info {
                Some(info) => Some(IndexedStr::Concrete(Cow::Borrowed(info))),
                None => None
            },
            host: IndexedStr::Concrete(Cow::Borrowed(host)),
            port,
        }
    }

    /// Parses the string `string` into an `Authority`. Parsing will never
    /// allocate. Returns an `Error` if `string` is not a valid authority URI.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::uri::Authority;
    ///
    /// // Parse a valid authority URI.
    /// let uri = Authority::parse("user:pass@host").expect("valid URI");
    /// assert_eq!(uri.user_info(), Some("user:pass"));
    /// assert_eq!(uri.host(), "host");
    /// assert_eq!(uri.port(), None);
    ///
    /// // Invalid authority URIs fail to parse.
    /// Authority::parse("https://rocket.rs").expect_err("invalid authority");
    ///
    /// // Prefer to use `uri!()` when the input is statically known:
    /// let uri = uri!("user:pass@host");
    /// assert_eq!(uri.user_info(), Some("user:pass"));
    /// assert_eq!(uri.host(), "host");
    /// assert_eq!(uri.port(), None);
    /// ```
    pub fn parse(string: &'a str) -> Result<Authority<'a>, Error<'a>> {
        crate::parse::uri::authority_from_str(string)
    }

    /// Parses the string `string` into an `Authority`. Parsing never allocates
    /// on success. May allocate on error.
    ///
    /// This method should be used instead of [`Authority::parse()`] when
    /// the source URI is already a `String`. Returns an `Error` if `string` is
    /// not a valid authority URI.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Authority;
    ///
    /// let source = format!("rocket.rs:8000");
    /// let uri = Authority::parse_owned(source).expect("valid URI");
    /// assert!(uri.user_info().is_none());
    /// assert_eq!(uri.host(), "rocket.rs");
    /// assert_eq!(uri.port(), Some(8000));
    /// ```
    pub fn parse_owned(string: String) -> Result<Authority<'static>, Error<'static>> {
        let authority = Authority::parse(&string).map_err(|e| e.into_owned())?;
        debug_assert!(authority.source.is_some(), "Authority parsed w/o source");

        let authority = Authority {
            host: authority.host.into_owned(),
            user_info: authority.user_info.into_owned(),
            port: authority.port,
            source: Some(Cow::Owned(string)),
        };

        Ok(authority)
    }

    /// Returns the user info part of the authority URI, if there is one.
    ///
    /// # Example
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// let uri = uri!("username:password@host");
    /// assert_eq!(uri.user_info(), Some("username:password"));
    /// ```
    pub fn user_info(&self) -> Option<&str> {
        self.user_info.as_ref().map(|u| u.from_cow_source(&self.source))
    }

    /// Returns the host part of the authority URI.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// let uri = uri!("domain.com:123");
    /// assert_eq!(uri.host(), "domain.com");
    ///
    /// let uri = uri!("username:password@host:123");
    /// assert_eq!(uri.host(), "host");
    ///
    /// let uri = uri!("username:password@[1::2]:123");
    /// assert_eq!(uri.host(), "[1::2]");
    /// ```
    #[inline(always)]
    pub fn host(&self) -> &str {
        self.host.from_cow_source(&self.source)
    }

    /// Returns the port part of the authority URI, if there is one.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// // With a port.
    /// let uri = uri!("username:password@host:123");
    /// assert_eq!(uri.port(), Some(123));
    ///
    /// let uri = uri!("domain.com:8181");
    /// assert_eq!(uri.port(), Some(8181));
    ///
    /// // Without a port.
    /// let uri = uri!("username:password@host");
    /// assert_eq!(uri.port(), None);
    /// ```
    #[inline(always)]
    pub fn port(&self) -> Option<u16> {
        self.port
    }
}

impl_serde!(Authority<'a>, "an authority-form URI");

impl_traits!(Authority, user_info, host, port);

impl Display for Authority<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(user_info) = self.user_info() {
            write!(f, "{}@", user_info)?;
        }

        self.host().fmt(f)?;
        if let Some(port) = self.port {
            write!(f, ":{}", port)?;
        }

        Ok(())
    }
}
