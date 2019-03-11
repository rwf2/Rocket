use std::borrow::{Borrow, Cow};
use std::ops::Deref;
use std::str::FromStr;
use std::fmt;

use header::Header;
use media_type::{MediaType, Source};
use ext::IntoCollection;
use hyper::mime::Mime;

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
/// # #[allow(unused_variables)]
/// let response = Response::build().header(ContentType::HTML).finalize();
/// ```
#[derive(Debug, Clone, PartialEq, Hash)]
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

    /// Creates a new `ContentType` with top-level type `top`, subtype `sub`,
    /// and parameters `ps`. This should _only_ be used to construct uncommon or
    /// custom content types. Use an associated constant for everything else.
    ///
    /// # Example
    ///
    /// Create a custom `application/x-id; id=1` content type:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::ContentType;
    ///
    /// let id = ContentType::with_params("application", "x-id", ("id", "1"));
    /// assert_eq!(id.to_string(), "application/x-id; id=1".to_string());
    /// ```
    ///
    /// Create a custom `text/person; name=bob; weight=175` content type:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::ContentType;
    ///
    /// let params = vec![("name", "bob"), ("ref", "2382")];
    /// let mt = ContentType::with_params("text", "person", params);
    /// assert_eq!(mt.to_string(), "text/person; name=bob; ref=2382".to_string());
    /// ```
    #[inline]
    pub fn with_params<T, S, K, V, P>(top: T, sub: S, ps: P) -> ContentType
        where T: Into<Cow<'static, str>>, S: Into<Cow<'static, str>>,
              K: Into<Cow<'static, str>>, V: Into<Cow<'static, str>>,
              P: IntoCollection<(K, V)>
    {
        ContentType(MediaType::with_params(top, sub, ps))
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

#[doc(hidden)]
impl<T: Borrow<Mime>> From<T> for ContentType {
    #[inline(always)]
    default fn from(mime: T) -> ContentType {
        let mime: Mime = mime.borrow().clone();
        ContentType::from(mime)
    }
}

#[doc(hidden)]
impl From<Mime> for ContentType {
    #[inline]
    fn from(mime: Mime) -> ContentType {
        // soooo inefficient.
        let params = mime.2.into_iter()
            .map(|(attr, value)| (attr.to_string(), value.to_string()))
            .collect::<Vec<_>>();

        ContentType::with_params(mime.0.to_string(), mime.1.to_string(), params)
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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Creates a new `Header` with name `Content-Type` and the value set to the
/// HTTP rendering of this Content-Type.
impl Into<Header<'static>> for ContentType {
    #[inline(always)]
    fn into(self) -> Header<'static> {
        // FIXME: For known media types, don't do `to_string`. Store the whole
        // string as a `source` and have a way to know that the source is
        // everything. That removes the allocation here. Then, in
        // `MediaType::fmt`, write the source string out directly as well.
        //
        // We could also use an `enum` for MediaType. But that kinda sucks. But
        // maybe it's what we want.
        if let Source::Known(src) = self.0.source {
            Header::new("Content-Type", src)
        } else {
            Header::new("Content-Type", self.to_string())
        }
    }
}
