use std::borrow::Cow;
use std::ops::Deref;
use std::str::FromStr;
use std::fmt;

use crate::header::{Header, MediaType};
use crate::uncased::UncasedStr;
use crate::ext::IntoCollection;

/// Representation of HTTP Content-Types.
///
/// # Usage
///
/// `ContentType`s should rarely be created directly. Instead, an associated
/// constant should be used; one is declared for most commonly used content
/// types.
///
/// ## Example
///
/// A Content-Type of `text/html; charset=utf-8` can be instantiated via the
/// `HTML` constant:
///
/// ```rust
/// # extern crate rocket;
/// use rocket::http::ContentType;
///
/// # #[allow(unused_variables)]
/// let html = ContentType::HTML;
/// ```
///
/// # Header
///
/// `ContentType` implements `Into<Header>`. As such, it can be used in any
/// context where an `Into<Header>` is expected:
///
/// ```rust
/// # extern crate rocket;
/// use rocket::http::ContentType;
/// use rocket::response::Response;
///
/// let response = Response::build().header(ContentType::HTML).finalize();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentType(pub MediaType);

macro_rules! content_types {
    ($($name:ident ($check:ident): $str:expr, $t:expr,
        $s:expr $(; $k:expr => $v:expr)*,)+) => {
    $(
        docify!([
            Content Type for @{"**"}! @{$str}! @{"**"}!: @{"`"} @{$t}! @[/]! @{$s}!
            $(; @{$k}! @[=]! @{$v}!)* @{"`"}!.
        ];
            #[allow(non_upper_case_globals)]
            pub const $name: ContentType = ContentType(MediaType::$name);
        );
    )+
}}

macro_rules! from_extension {
    ($($ext:expr => $name:ident,)*) => (
    docify!([
        Returns the @[Content-Type] associated with the extension @code{ext}.
        Not all extensions are recognized. If an extensions is not recognized,
        @code{None} is returned. The currently recognized extensions are:

        @nl
        $(* @{$ext} - @{"`ContentType::"}! @[$name]! @{"`"} @nl)*
        @nl

        This list is likely to grow. Extensions are matched
        @[case-insensitively.]
    ];
        /// # Example
        ///
        /// Recognized content types:
        ///
        /// ```rust
        /// # extern crate rocket;
        /// use rocket::http::ContentType;
        ///
        /// let xml = ContentType::from_extension("xml");
        /// assert_eq!(xml, Some(ContentType::XML));
        ///
        /// let xml = ContentType::from_extension("XML");
        /// assert_eq!(xml, Some(ContentType::XML));
        /// ```
        ///
        /// An unrecognized content type:
        ///
        /// ```rust
        /// # extern crate rocket;
        /// use rocket::http::ContentType;
        ///
        /// let foo = ContentType::from_extension("foo");
        /// assert!(foo.is_none());
        /// ```
        #[inline]
        pub fn from_extension(ext: &str) -> Option<ContentType> {
            MediaType::from_extension(ext).map(ContentType)
        }
    );)
}

macro_rules! extension {
    ($($ext:expr => $name:ident,)*) => (
    docify!([
        Returns the most common file extension associated with the
        @[Content-Type] @code{self} if it is known. Otherwise, returns
        @code{None}. The currently recognized extensions are identical to those
        in @{"[`ContentType::from_extension()`]"} with the @{"most common"}
        extension being the first extension appearing in the list for a given
        @[Content-Type].
    ];
        /// # Example
        ///
        /// Known extension:
        ///
        /// ```rust
        /// # extern crate rocket;
        /// use rocket::http::ContentType;
        ///
        /// assert_eq!(ContentType::JSON.extension().unwrap(), "json");
        /// assert_eq!(ContentType::JPEG.extension().unwrap(), "jpeg");
        /// assert_eq!(ContentType::JPEG.extension().unwrap(), "JPEG");
        /// assert_eq!(ContentType::PDF.extension().unwrap(), "pdf");
        /// ```
        ///
        /// An unknown extension:
        ///
        /// ```rust
        /// # extern crate rocket;
        /// use rocket::http::ContentType;
        ///
        /// let foo = ContentType::new("foo", "bar");
        /// assert!(foo.extension().is_none());
        /// ```
        #[inline]
        pub fn extension(&self) -> Option<&UncasedStr> {
            $(if self == &ContentType::$name { return Some($ext.into()) })*
            None
        }
    );)
}

macro_rules! parse_flexible {
    ($($short:expr => $name:ident,)*) => (
    docify!([
        Flexibly parses @code{name} into a @code{ContentType}. The parse is
        @[_flexible_] because, in addition to stricly correct content types, it
        recognizes the following shorthands:

        @nl
        $(* $short - @{"`ContentType::"}! @[$name]! @{"`"} @nl)*
        @nl
    ];
        ///
        /// For regular parsing, use the
        /// [`ContentType::from_str()`](#impl-FromStr) method.
        ///
        /// # Example
        ///
        /// Using a shorthand:
        ///
        /// ```rust
        /// # extern crate rocket;
        /// use rocket::http::ContentType;
        ///
        /// let html = ContentType::parse_flexible("html");
        /// assert_eq!(html, Some(ContentType::HTML));
        ///
        /// let json = ContentType::parse_flexible("json");
        /// assert_eq!(json, Some(ContentType::JSON));
        /// ```
        ///
        /// Using the full content-type:
        ///
        /// ```rust
        /// # extern crate rocket;
        /// use rocket::http::ContentType;
        ///
        /// let html = ContentType::parse_flexible("text/html; charset=utf-8");
        /// assert_eq!(html, Some(ContentType::HTML));
        ///
        /// let json = ContentType::parse_flexible("application/json");
        /// assert_eq!(json, Some(ContentType::JSON));
        ///
        /// let custom = ContentType::parse_flexible("application/x+custom");
        /// assert_eq!(custom, Some(ContentType::new("application", "x+custom")));
        /// ```
        ///
        /// An unrecognized content-type:
        ///
        /// ```rust
        /// # extern crate rocket;
        /// use rocket::http::ContentType;
        ///
        /// let foo = ContentType::parse_flexible("foo");
        /// assert_eq!(foo, None);
        ///
        /// let bar = ContentType::parse_flexible("foo/bar/baz");
        /// assert_eq!(bar, None);
        /// ```
        #[inline]
        pub fn parse_flexible(name: &str) -> Option<ContentType> {
            MediaType::parse_flexible(name).map(ContentType)
        }
    );)
}

impl ContentType {
    /// Creates a new `ContentType` with top-level type `top` and subtype `sub`.
    /// This should _only_ be used to construct uncommon or custom content
    /// types. Use an associated constant for everything else.
    ///
    /// # Example
    ///
    /// Create a custom `application/x-person` content type:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::ContentType;
    ///
    /// let custom = ContentType::new("application", "x-person");
    /// assert_eq!(custom.top(), "application");
    /// assert_eq!(custom.sub(), "x-person");
    /// ```
    #[inline(always)]
    pub fn new<T, S>(top: T, sub: S) -> ContentType
        where T: Into<Cow<'static, str>>, S: Into<Cow<'static, str>>
    {
        ContentType(MediaType::new(top, sub))
    }

    known_shorthands!(parse_flexible);

    known_extensions!(from_extension);

    /// Sets the parameters `parameters` on `self`.
    ///
    /// # Example
    ///
    /// Create a custom `application/x-id; id=1` media type:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::ContentType;
    ///
    /// let id = ContentType::new("application", "x-id").with_params(("id", "1"));
    /// assert_eq!(id.to_string(), "application/x-id; id=1".to_string());
    /// ```
    ///
    /// Create a custom `text/person; name=bob; weight=175` media type:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::ContentType;
    ///
    /// let mt = ContentType::new("text", "person")
    ///     .with_params([("name", "bob"), ("ref", "2382")]);
    ///
    /// assert_eq!(mt.to_string(), "text/person; name=bob; ref=2382".to_string());
    /// ```
    pub fn with_params<K, V, P>(self, parameters: P) -> ContentType
        where K: Into<Cow<'static, str>>,
              V: Into<Cow<'static, str>>,
              P: IntoCollection<(K, V)>
    {
        ContentType(self.0.with_params(parameters))
    }

    /// Borrows the inner `MediaType` of `self`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::{ContentType, MediaType};
    ///
    /// let http = ContentType::HTML;
    /// let media_type = http.media_type();
    /// ```
    #[inline(always)]
    pub fn media_type(&self) -> &MediaType {
        &self.0
    }

    known_extensions!(extension);

    known_media_types!(content_types);
}

impl Default for ContentType {
    /// Returns a ContentType of `Any`, or `*/*`.
    #[inline(always)]
    fn default() -> ContentType {
        ContentType::Any
    }
}

impl Deref for ContentType {
    type Target = MediaType;

    #[inline(always)]
    fn deref(&self) -> &MediaType {
        &self.0
    }
}

impl FromStr for ContentType {
    type Err = String;

    /// Parses a `ContentType` from a given Content-Type header value.
    ///
    /// # Examples
    ///
    /// Parsing an `application/json`:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use std::str::FromStr;
    /// use rocket::http::ContentType;
    ///
    /// let json = ContentType::from_str("application/json").unwrap();
    /// assert!(json.is_known());
    /// assert_eq!(json, ContentType::JSON);
    /// ```
    ///
    /// Parsing a content type extension:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use std::str::FromStr;
    /// use rocket::http::ContentType;
    ///
    /// let custom = ContentType::from_str("application/x-custom").unwrap();
    /// assert!(!custom.is_known());
    /// assert_eq!(custom.top(), "application");
    /// assert_eq!(custom.sub(), "x-custom");
    /// ```
    ///
    /// Parsing an invalid Content-Type value:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use std::str::FromStr;
    /// use rocket::http::ContentType;
    ///
    /// let custom = ContentType::from_str("application//x-custom");
    /// assert!(custom.is_err());
    /// ```
    #[inline(always)]
    fn from_str(raw: &str) -> Result<ContentType, String> {
        MediaType::from_str(raw).map(ContentType)
    }
}

impl From<MediaType> for ContentType {
    fn from(media_type: MediaType) -> Self {
        ContentType(media_type)
    }
}

impl fmt::Display for ContentType {
    /// Formats the ContentType as an HTTP Content-Type value.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::ContentType;
    ///
    /// let ct = format!("{}", ContentType::JSON);
    /// assert_eq!(ct, "application/json");
    /// ```
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Creates a new `Header` with name `Content-Type` and the value set to the
/// HTTP rendering of this Content-Type.
impl From<ContentType> for Header<'static> {
    #[inline(always)]
    fn from(content_type: ContentType) -> Self {
        if let Some(src) = content_type.known_source() {
            Header::new("Content-Type", src)
        } else {
            Header::new("Content-Type", content_type.to_string())
        }
    }
}
