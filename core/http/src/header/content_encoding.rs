use std::borrow::Cow;
use std::ops::Deref;
use std::str::FromStr;
use std::fmt;

use crate::header::Header;
use crate::ContentCoding;

/// Representation of HTTP Content-Encoding.
///
/// # Usage
///
/// `ContentEncoding`s should rarely be created directly. Instead, an associated
/// constant should be used; one is declared for most commonly used content
/// types.
///
/// ## Example
///
/// A Content-Encoding of `gzip` can be instantiated via the
/// `GZIP` constant:
///
/// ```rust
/// # extern crate rocket;
/// use rocket::http::ContentEncoding;
///
/// # #[allow(unused_variables)]
/// let html = ContentEncoding::GZIP;
/// ```
///
/// # Header
///
/// `ContentEncoding` implements `Into<Header>`. As such, it can be used in any
/// context where an `Into<Header>` is expected:
///
/// ```rust
/// # extern crate rocket;
/// use rocket::http::ContentEncoding;
/// use rocket::response::Response;
///
/// let response = Response::build().header(ContentEncoding::GZIP).finalize();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentEncoding(pub ContentCoding);

macro_rules! content_encodings {
    ($($name:ident ($check:ident): $str:expr, $c:expr,)+) => {
    $(

        /// Content Encoding for
        #[doc = concat!("**", $str, "**: ")]
        #[doc = concat!("`", $c, "`")]

        #[allow(non_upper_case_globals)]
        pub const $name: ContentEncoding = ContentEncoding(ContentCoding::$name);
    )+
}}

impl ContentEncoding {
    /// Creates a new `ContentEncoding` with `coding`.
    /// This should _only_ be used to construct uncommon or custom content
    /// types. Use an associated constant for everything else.
    ///
    /// # Example
    ///
    /// Create a custom `foo` content encoding:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::ContentEncoding;
    ///
    /// let custom = ContentEncoding::new("foo");
    /// assert_eq!(custom.content_coding(), "foo");
    /// ```
    #[inline(always)]
    pub fn new<S>(coding: S) -> ContentEncoding
        where S: Into<Cow<'static, str>>
    {
        ContentEncoding(ContentCoding::new(coding))
    }

    /// Borrows the inner `ContentCoding` of `self`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::{ContentEncoding, ContentCoding};
    ///
    /// let http = ContentEncoding::GZIP;
    /// let content_coding = http.content_coding();
    /// ```
    #[inline(always)]
    pub fn content_coding(&self) -> &ContentCoding {
        &self.0
    }

    known_content_codings!(content_encodings);
}

impl Default for ContentEncoding {
    /// Returns a ContentEncoding of `Any`, or `*`.
    #[inline(always)]
    fn default() -> ContentEncoding {
        ContentEncoding::Any
    }
}

impl Deref for ContentEncoding {
    type Target = ContentCoding;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for ContentEncoding {
    type Err = String;

    /// Parses a `ContentEncoding` from a given Content-Encoding header value.
    ///
    /// # Examples
    ///
    /// Parsing a `gzip`:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use std::str::FromStr;
    /// use rocket::http::ContentEncoding;
    ///
    /// let gzip = ContentEncoding::from_str("gzip").unwrap();
    /// assert!(gzip.is_known());
    /// assert_eq!(gzip, ContentEncoding::GZIP);
    /// ```
    ///
    /// Parsing an invalid Content-Encoding value:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use std::str::FromStr;
    /// use rocket::http::ContentEncoding;
    ///
    /// let custom = ContentEncoding::from_str("12ec/.322r");
    /// assert!(custom.is_err());
    /// ```
    #[inline(always)]
    fn from_str(raw: &str) -> Result<ContentEncoding, String> {
        ContentCoding::from_str(raw).map(ContentEncoding)
    }
}

impl From<ContentCoding> for ContentEncoding {
    fn from(content_coding: ContentCoding) -> Self {
        ContentEncoding(content_coding)
    }
}

impl fmt::Display for ContentEncoding {
    /// Formats the ContentEncoding as an HTTP Content-Encoding value.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::ContentEncoding;
    ///
    /// let cc = format!("{}", ContentEncoding::GZIP);
    /// assert_eq!(cc, "gzip");
    /// ```
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Creates a new `Header` with name `Content-Encoding` and the value set to the
/// HTTP rendering of this Content-Encoding.
impl From<ContentEncoding> for Header<'static> {
    #[inline(always)]
    fn from(content_encoding: ContentEncoding) -> Self {
        if let Some(src) = content_encoding.known_source() {
            Header::new("Content-Encoding", src)
        } else {
            Header::new("Content-Encoding", content_encoding.to_string())
        }
    }
}
