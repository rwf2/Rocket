use std::fmt::{self, Display};
use std::borrow::Cow;

use crate::ext::IntoOwned;
use crate::parse::{Indexed, IndexedStr};
use crate::uri::{as_utf8_unchecked, Error};

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
#[derive(Debug, Clone)]
pub struct Authority<'a> {
    source: Option<Cow<'a, str>>,
    user_info: Option<IndexedStr<'a>>,
    host: Host<IndexedStr<'a>>,
    port: Option<u16>,
}

#[derive(Debug, Clone)]
crate enum Host<T> {
    Bracketed(T),
    Raw(T)
}

impl<T: IntoOwned> IntoOwned for Host<T> {
    type Owned = Host<T::Owned>;

    fn into_owned(self) -> Self::Owned {
        self.map_inner(IntoOwned::into_owned)
    }
}

impl IntoOwned for Authority<'_> {
    type Owned = Authority<'static>;

    fn into_owned(self) -> Authority<'static> {
        Authority {
            source: self.source.into_owned(),
            user_info: self.user_info.into_owned(),
            host: self.host.into_owned(),
            port: self.port
        }
    }
}

impl<'a> Authority<'a> {
    crate unsafe fn raw(
        source: Cow<'a, [u8]>,
        user_info: Option<Indexed<'a, [u8]>>,
        host: Host<Indexed<'a, [u8]>>,
        port: Option<u16>
    ) -> Authority<'a> {
        Authority {
            source: Some(as_utf8_unchecked(source)),
            user_info: user_info.map(|u| u.coerce()),
            host: host.map_inner(|inner| inner.coerce()),
            port: port
        }
    }

    #[cfg(test)]
    crate fn new(
        user_info: Option<&'a str>,
        host: Host<&'a str>,
        port: Option<u16>
    ) -> Authority<'a> {
        Authority {
            source: None,
            user_info: user_info.map(|u| u.into()),
            host: host.map_inner(|inner| inner.into()),
            port: port
        }
    }

    /// Parses the string `string` into an `Authority`. Parsing will never
    /// allocate. Returns an `Error` if `string` is not a valid authority URI.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Authority;
    ///
    /// // Parse a valid authority URI.
    /// let uri = Authority::parse("user:pass@host").expect("valid URI");
    /// assert_eq!(uri.user_info(), Some("user:pass"));
    /// assert_eq!(uri.host(), "host");
    /// assert_eq!(uri.port(), None);
    ///
    /// // Invalid authority URIs fail to parse.
    /// Authority::parse("http://google.com").expect_err("invalid authority");
    /// ```
    pub fn parse(string: &'a str) -> Result<Authority<'a>, Error<'a>> {
        crate::parse::uri::authority_from_str(string)
    }

    /// Returns the user info part of the authority URI, if there is one.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Authority;
    ///
    /// let uri = Authority::parse("username:password@host").unwrap();
    /// assert_eq!(uri.user_info(), Some("username:password"));
    /// ```
    pub fn user_info(&self) -> Option<&str> {
        self.user_info.as_ref().map(|u| u.from_cow_source(&self.source))
    }

    /// Returns the host part of the authority URI.
    ///
    ///
    /// If the host was provided in brackets (such as for IPv6 addresses), the
    /// brackets will not be part of the returned string.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Authority;
    ///
    /// let uri = Authority::parse("domain.com:123").unwrap();
    /// assert_eq!(uri.host(), "domain.com");
    ///
    /// let uri = Authority::parse("username:password@host:123").unwrap();
    /// assert_eq!(uri.host(), "host");
    ///
    /// let uri = Authority::parse("username:password@[1::2]:123").unwrap();
    /// assert_eq!(uri.host(), "1::2");
    /// ```
    #[inline(always)]
    pub fn host(&self) -> &str {
        self.host.inner().from_cow_source(&self.source)
    }

    /// Returns the port part of the authority URI, if there is one.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Authority;
    ///
    /// // With a port.
    /// let uri = Authority::parse("username:password@host:123").unwrap();
    /// assert_eq!(uri.port(), Some(123));
    ///
    /// let uri = Authority::parse("domain.com:8181").unwrap();
    /// assert_eq!(uri.port(), Some(8181));
    ///
    /// // Without a port.
    /// let uri = Authority::parse("username:password@host").unwrap();
    /// assert_eq!(uri.port(), None);
    /// ```
    #[inline(always)]
    pub fn port(&self) -> Option<u16> {
        self.port
    }
}

impl<'b> PartialEq<Authority<'b>> for Authority<'_> {
    fn eq(&self, other: &Authority<'b>) -> bool {
        self.user_info() == other.user_info()
            && self.host() == other.host()
            && self.host.is_bracketed() == other.host.is_bracketed()
            && self.port() == other.port()
    }
}

impl Display for Authority<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(user_info) = self.user_info() {
            write!(f, "{}@", user_info)?;
        }

        match self.host {
            Host::Bracketed(_) => write!(f, "[{}]", self.host())?,
            Host::Raw(_) => write!(f, "{}", self.host())?
        }

        if let Some(port) = self.port {
            write!(f, ":{}", port)?;
        }

        Ok(())
    }
}

impl<T> Host<T> {
    #[inline]
    fn inner(&self) -> &T {
        match *self {
            Host::Bracketed(ref inner) | Host::Raw(ref inner) => inner
        }
    }

    #[inline]
    fn is_bracketed(&self) -> bool {
        match *self {
            Host::Bracketed(_) => true,
            _ => false
        }
    }

    #[inline]
    fn map_inner<F, U>(self, f: F) -> Host<U>
        where F: FnOnce(T) -> U
    {
        match self {
            Host::Bracketed(inner) => Host::Bracketed(f(inner)),
            Host::Raw(inner) => Host::Raw(f(inner))
        }
    }
}
