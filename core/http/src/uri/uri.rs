use std::fmt::{self, Display};
use std::convert::From;
use std::borrow::Cow;
use std::str::Utf8Error;
use std::convert::TryFrom;

use crate::ext::IntoOwned;
use crate::parse::Indexed;
use crate::uri::{Origin, Authority, Absolute, Error};
use crate::uri::encoding::{percent_encode, DEFAULT_ENCODE_SET};

/// An `enum` encapsulating any of the possible URI variants.
///
/// # Usage
///
/// In Rocket, this type will rarely be used directly. Instead, you will
/// typically encounter URIs via the [`Origin`] type. This is because all
/// incoming requests contain origin-type URIs.
///
/// Nevertheless, the `Uri` type is typically enountered as a conversion target.
/// In particular, you will likely see generic bounds of the form: `T:
/// TryInto<Uri>` (for instance, in [`Redirect`](rocket::response::Redirect)
/// methods). This means that you can provide any type `T` that implements
/// `TryInto<Uri>`, or, equivalently, any type `U` for which `Uri` implements
/// `TryFrom<U>` or `From<U>`. These include `&str` and `String`, [`Origin`],
/// [`Authority`], and [`Absolute`].
///
/// ## Parsing
///
/// The `Uri` type implements a full, zero-allocation, zero-copy [RFC 7230]
/// compliant parser. To parse an `&str` into a `Uri`, use the [`Uri::parse()`]
/// method. Alternatively, you may also use the `TryFrom<&str>` and
/// `TryFrom<String>` trait implementation. To inspect the parsed type, match on
/// the resulting `enum` and use the methods of the internal structure.
///
/// [RFC 7230]: https://tools.ietf.org/html/rfc7230
///
/// ## Percent Encoding/Decoding
///
/// This type also provides the following percent encoding/decoding helper
/// methods: [`Uri::percent_encode()`], [`Uri::percent_decode()`], and
/// [`Uri::percent_decode_lossy()`].
///
/// [`Origin`]: crate::uri::Origin
/// [`Authority`]: crate::uri::Authority
/// [`Absolute`]: crate::uri::Absolute
/// [`Uri::parse()`]: crate::uri::Uri::parse()
/// [`Uri::percent_encode()`]: crate::uri::Uri::percent_encode()
/// [`Uri::percent_decode()`]: crate::uri::Uri::percent_decode()
/// [`Uri::percent_decode_lossy()`]: crate::uri::Uri::percent_decode_lossy()
#[derive(Debug, PartialEq, Clone)]
pub enum Uri<'a> {
    /// An origin URI.
    Origin(Origin<'a>),
    /// An authority URI.
    Authority(Authority<'a>),
    /// An absolute URI.
    Absolute(Absolute<'a>),
    /// An asterisk: exactly `*`.
    Asterisk,
}

impl<'a> Uri<'a> {
    #[inline]
    crate unsafe fn raw_absolute(
        source: Cow<'a, [u8]>,
        scheme: Indexed<'a, [u8]>,
        path: Indexed<'a, [u8]>,
        query: Option<Indexed<'a, [u8]>>,
    ) -> Uri<'a> {
        let origin = Origin::raw(source.clone(), path, query);
        Uri::Absolute(Absolute::raw(source.clone(), scheme, None, Some(origin)))
    }

    /// Parses the string `string` into a `Uri`. Parsing will never allocate.
    /// Returns an `Error` if `string` is not a valid URI.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Uri;
    ///
    /// // Parse a valid origin URI (note: in practice, use `Origin::parse()`).
    /// let uri = Uri::parse("/a/b/c?query").expect("valid URI");
    /// let origin = uri.origin().expect("origin URI");
    /// assert_eq!(origin.path(), "/a/b/c");
    /// assert_eq!(origin.query(), Some("query"));
    ///
    /// // Invalid URIs fail to parse.
    /// Uri::parse("foo bar").expect_err("invalid URI");
    /// ```
    pub fn parse(string: &'a str) -> Result<Uri<'a>, Error<'_>> {
        crate::parse::uri::from_str(string)
    }

    /// Returns the internal instance of `Origin` if `self` is a `Uri::Origin`.
    /// Otherwise, returns `None`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Uri;
    ///
    /// let uri = Uri::parse("/a/b/c?query").expect("valid URI");
    /// assert!(uri.origin().is_some());
    ///
    /// let uri = Uri::parse("http://google.com").expect("valid URI");
    /// assert!(uri.origin().is_none());
    /// ```
    pub fn origin(&self) -> Option<&Origin<'a>> {
        match self {
            Uri::Origin(ref inner) => Some(inner),
            _ => None
        }
    }

    /// Returns the internal instance of `Authority` if `self` is a
    /// `Uri::Authority`. Otherwise, returns `None`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Uri;
    ///
    /// let uri = Uri::parse("user:pass@domain.com").expect("valid URI");
    /// assert!(uri.authority().is_some());
    ///
    /// let uri = Uri::parse("http://google.com").expect("valid URI");
    /// assert!(uri.authority().is_none());
    /// ```
    pub fn authority(&self) -> Option<&Authority<'a>> {
        match self {
            Uri::Authority(ref inner) => Some(inner),
            _ => None
        }
    }

    /// Returns the internal instance of `Absolute` if `self` is a
    /// `Uri::Absolute`. Otherwise, returns `None`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Uri;
    ///
    /// let uri = Uri::parse("http://google.com").expect("valid URI");
    /// assert!(uri.absolute().is_some());
    ///
    /// let uri = Uri::parse("/path").expect("valid URI");
    /// assert!(uri.absolute().is_none());
    /// ```
    pub fn absolute(&self) -> Option<&Absolute<'a>> {
        match self {
            Uri::Absolute(ref inner) => Some(inner),
            _ => None
        }
    }

    /// Returns a URL-encoded version of the string. Any reserved characters are
    /// percent-encoded.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Uri;
    ///
    /// let encoded = Uri::percent_encode("hello?a=<b>hi</b>");
    /// assert_eq!(encoded, "hello%3Fa%3D%3Cb%3Ehi%3C%2Fb%3E");
    /// ```
    pub fn percent_encode(string: &str) -> Cow<'_, str> {
        percent_encode::<DEFAULT_ENCODE_SET>(string)
    }

    /// Returns a URL-decoded version of the string. If the percent encoded
    /// values are not valid UTF-8, an `Err` is returned.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Uri;
    ///
    /// let decoded = Uri::percent_decode("/Hello%2C%20world%21".as_bytes());
    /// assert_eq!(decoded.unwrap(), "/Hello, world!");
    /// ```
    pub fn percent_decode(string: &[u8]) -> Result<Cow<'_, str>, Utf8Error> {
        let decoder = percent_encoding::percent_decode(string);
        decoder.decode_utf8()
    }

    /// Returns a URL-decoded version of the path. Any invalid UTF-8
    /// percent-encoded byte sequences will be replaced � U+FFFD, the
    /// replacement character.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Uri;
    ///
    /// let decoded = Uri::percent_decode_lossy("/Hello%2C%20world%21".as_bytes());
    /// assert_eq!(decoded, "/Hello, world!");
    /// ```
    pub fn percent_decode_lossy(string: &[u8]) -> Cow<'_, str> {
        let decoder = percent_encoding::percent_decode(string);
        decoder.decode_utf8_lossy()
    }
}

crate unsafe fn as_utf8_unchecked(input: Cow<'_, [u8]>) -> Cow<'_, str> {
    match input {
        Cow::Borrowed(bytes) => Cow::Borrowed(std::str::from_utf8_unchecked(bytes)),
        Cow::Owned(bytes) => Cow::Owned(String::from_utf8_unchecked(bytes))
    }
}

impl<'a> TryFrom<&'a str> for Uri<'a> {
    type Error = Error<'a>;

    #[inline]
    fn try_from(string: &'a str) -> Result<Uri<'a>, Self::Error> {
        Uri::parse(string)
    }
}

impl TryFrom<String> for Uri<'static> {
    type Error = Error<'static>;

    #[inline]
    fn try_from(string: String) -> Result<Uri<'static>, Self::Error> {
        // TODO: Potentially optimize this like `Origin::parse_owned`.
        Uri::parse(&string)
            .map(|u| u.into_owned())
            .map_err(|e| e.into_owned())
    }
}

impl IntoOwned for Uri<'_> {
    type Owned = Uri<'static>;

    fn into_owned(self) -> Uri<'static> {
        match self {
            Uri::Origin(origin) => Uri::Origin(origin.into_owned()),
            Uri::Authority(authority) => Uri::Authority(authority.into_owned()),
            Uri::Absolute(absolute) => Uri::Absolute(absolute.into_owned()),
            Uri::Asterisk => Uri::Asterisk
        }
    }
}

impl Display for Uri<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Uri::Origin(ref origin) => write!(f, "{}", origin),
            Uri::Authority(ref authority) => write!(f, "{}", authority),
            Uri::Absolute(ref absolute) => write!(f, "{}", absolute),
            Uri::Asterisk => write!(f, "*")
        }
    }
}

macro_rules! impl_uri_from {
    ($type:ident) => (
        impl<'a> From<$type<'a>> for Uri<'a> {
            fn from(other: $type<'a>) -> Uri<'a> {
                Uri::$type(other)
            }
        }
    )
}

impl_uri_from!(Origin);
impl_uri_from!(Authority);
impl_uri_from!(Absolute);
