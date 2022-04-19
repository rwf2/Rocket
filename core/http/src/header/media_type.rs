use std::borrow::{Cow, Borrow};
use std::str::FromStr;
use std::fmt;
use std::hash::{Hash, Hasher};

use either::Either;

use crate::ext::IntoCollection;
use crate::uncased::UncasedStr;
use crate::parse::{Indexed, IndexedStr, parse_media_type};

use smallvec::SmallVec;

/// An HTTP media type.
///
/// # Usage
///
/// A `MediaType` should rarely be used directly. Instead, one is typically used
/// indirectly via types like [`Accept`](crate::Accept) and
/// [`ContentType`](crate::ContentType), which internally contain `MediaType`s.
/// Nonetheless, a `MediaType` can be created via the [`MediaType::new()`],
/// [`MediaType::with_params()`], and [`MediaType::from_extension`()] methods.
/// The preferred method, however, is to create a `MediaType` via an associated
/// constant.
///
/// ## Example
///
/// A media type of `application/json` can be instantiated via the
/// [`MediaType::JSON`] constant:
///
/// ```rust
/// # extern crate rocket;
/// use rocket::http::MediaType;
///
/// let json = MediaType::JSON;
/// assert_eq!(json.top(), "application");
/// assert_eq!(json.sub(), "json");
///
/// let json = MediaType::new("application", "json");
/// assert_eq!(MediaType::JSON, json);
/// ```
///
/// # Comparison and Hashing
///
/// The `PartialEq` and `Hash` implementations for `MediaType` _do not_ take
/// into account parameters. This means that a media type of `text/html` is
/// equal to a media type of `text/html; charset=utf-8`, for instance. This is
/// typically the comparison that is desired.
///
/// If an exact comparison is desired that takes into account parameters, the
/// [`exact_eq()`](MediaType::exact_eq()) method can be used.
#[derive(Debug, Clone)]
pub struct MediaType {
    /// Storage for the entire media type string.
    pub(crate) source: Source,
    /// The top-level type.
    pub(crate) top: IndexedStr<'static>,
    /// The subtype.
    pub(crate) sub: IndexedStr<'static>,
    /// The parameters, if any.
    pub(crate) params: MediaParams
}

// FIXME: `Static` variant is needed for `const`. Need `const SmallVec::new`.
#[derive(Debug, Clone)]
pub(crate) enum MediaParams {
    Static(&'static [(&'static str, &'static str)]),
    Dynamic(SmallVec<[(IndexedStr<'static>, IndexedStr<'static>); 2]>)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Source {
    Known(&'static str),
    Custom(Cow<'static, str>),
    None
}

impl From<Cow<'static, str>> for Source {
    fn from(custom: Cow<'static, str>) -> Source {
        Source::Custom(custom)
    }
}

macro_rules! media_types {
    ($($name:ident ($check:ident): $str:expr, $t:expr,
        $s:expr $(; $k:expr => $v:expr)*,)+) => {
    $(
        docify!([
            Media Type for @{"**"}! @{$str}! @{"**"}!: @{"`"} @{$t}! @[/]! @{$s}!
            $(; @{$k}! @[=]! @{$v}!)* @{"`"}!.
        ];
            #[allow(non_upper_case_globals)]
            pub const $name: MediaType = MediaType::new_known(
                concat!($t, "/", $s, $("; ", $k, "=", $v),*),
                $t, $s, &[$(($k, $v)),*]
            );
        );
    )+

    /// Returns `true` if this MediaType is known to Rocket. In other words,
    /// returns `true` if there is an associated constant for `self`.
    pub fn is_known(&self) -> bool {
        if let Source::Known(_) = self.source {
            return true;
        }

        $(if self.$check() { return true })+
        false
    }

    $(
        docify!([
            Returns @code{true} if the @[top-level] and sublevel types of
            @code{self} are the same as those of @{"`MediaType::"}! $name
            @{"`"}!.
        ];
            #[inline(always)]
            pub fn $check(&self) -> bool {
                *self == MediaType::$name
            }
        );
    )+
}}

macro_rules! from_extension {
    ($($ext:expr => $name:ident,)*) => (
    docify!([
        Returns the @[Media-Type] associated with the extension @code{ext}. Not
        all extensions are recognized. If an extensions is not recognized,
        @code{None} is returned. The currently recognized extensions are:

        @nl
        $(* @{$ext} - @{"`MediaType::"}! @[$name]! @{"`"} @nl)*
        @nl

        This list is likely to grow. Extensions are matched
        @[case-insensitively.]
    ];
        /// # Example
        ///
        /// Recognized media types:
        ///
        /// ```rust
        /// # extern crate rocket;
        /// use rocket::http::MediaType;
        ///
        /// let xml = MediaType::from_extension("xml");
        /// assert_eq!(xml, Some(MediaType::XML));
        ///
        /// let xml = MediaType::from_extension("XML");
        /// assert_eq!(xml, Some(MediaType::XML));
        /// ```
        ///
        /// An unrecognized media type:
        ///
        /// ```rust
        /// # extern crate rocket;
        /// use rocket::http::MediaType;
        ///
        /// let foo = MediaType::from_extension("foo");
        /// assert!(foo.is_none());
        /// ```
        pub fn from_extension(ext: &str) -> Option<MediaType> {
            match ext {
                $(x if uncased::eq(x, $ext) => Some(MediaType::$name)),*,
                _ => None
            }
        }
    );)
}

macro_rules! extension {
    ($($ext:expr => $name:ident,)*) => (
    docify!([
        Returns the most common file extension associated with the @[Media-Type]
        @code{self} if it is known. Otherwise, returns @code{None}. The
        currently recognized extensions are identical to those in
        @{"[`MediaType::from_extension()`]"} with the @{"most common"} extension
        being the first extension appearing in the list for a given
        @[Media-Type].
    ];
        /// # Example
        ///
        /// Known extension:
        ///
        /// ```rust
        /// # extern crate rocket;
        /// use rocket::http::MediaType;
        ///
        /// assert_eq!(MediaType::JSON.extension().unwrap(), "json");
        /// assert_eq!(MediaType::JPEG.extension().unwrap(), "jpeg");
        /// assert_eq!(MediaType::JPEG.extension().unwrap(), "JPEG");
        /// assert_eq!(MediaType::PDF.extension().unwrap(), "pdf");
        /// ```
        ///
        /// An unknown extension:
        ///
        /// ```rust
        /// # extern crate rocket;
        /// use rocket::http::MediaType;
        ///
        /// let foo = MediaType::new("foo", "bar");
        /// assert!(foo.extension().is_none());
        /// ```
        #[inline]
        pub fn extension(&self) -> Option<&UncasedStr> {
            $(if self == &MediaType::$name { return Some($ext.into()) })*
            None
        }
    );)
}

macro_rules! parse_flexible {
    ($($short:expr => $name:ident,)*) => (
    docify!([
        Flexibly parses @code{name} into a @code{MediaType}. The parse is
        @[_flexible_] because, in addition to stricly correct media types, it
        recognizes the following shorthands:

        @nl
        $(* $short - @{"`MediaType::"}! @[$name]! @{"`"} @nl)*
        @nl
    ];
        /// For regular parsing, use the
        /// [`MediaType::from_str()`](#impl-FromStr) method.
        ///
        /// # Example
        ///
        /// Using a shorthand:
        ///
        /// ```rust
        /// # extern crate rocket;
        /// use rocket::http::MediaType;
        ///
        /// let html = MediaType::parse_flexible("html");
        /// assert_eq!(html, Some(MediaType::HTML));
        ///
        /// let json = MediaType::parse_flexible("json");
        /// assert_eq!(json, Some(MediaType::JSON));
        /// ```
        ///
        /// Using the full media type:
        ///
        /// ```rust
        /// # extern crate rocket;
        /// use rocket::http::MediaType;
        ///
        /// let html = MediaType::parse_flexible("text/html; charset=utf-8");
        /// assert_eq!(html, Some(MediaType::HTML));
        ///
        /// let json = MediaType::parse_flexible("application/json");
        /// assert_eq!(json, Some(MediaType::JSON));
        ///
        /// let custom = MediaType::parse_flexible("application/x+custom");
        /// assert_eq!(custom, Some(MediaType::new("application", "x+custom")));
        /// ```
        ///
        /// An unrecognized media type:
        ///
        /// ```rust
        /// # extern crate rocket;
        /// use rocket::http::MediaType;
        ///
        /// let foo = MediaType::parse_flexible("foo");
        /// assert_eq!(foo, None);
        ///
        /// let bar = MediaType::parse_flexible("foo/bar/baz");
        /// assert_eq!(bar, None);
        /// ```
        pub fn parse_flexible(name: &str) -> Option<MediaType> {
            match name {
                $(x if uncased::eq(x, $short) => Some(MediaType::$name)),*,
                _ => MediaType::from_str(name).ok(),
            }
        }
    );)
}

impl MediaType {
    /// Creates a new `MediaType` with top-level type `top` and subtype `sub`.
    /// This should _only_ be used to construct uncommon or custom media types.
    /// Use an associated constant for everything else.
    ///
    /// # Example
    ///
    /// Create a custom `application/x-person` media type:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::MediaType;
    ///
    /// let custom = MediaType::new("application", "x-person");
    /// assert_eq!(custom.top(), "application");
    /// assert_eq!(custom.sub(), "x-person");
    /// ```
    #[inline]
    pub fn new<T, S>(top: T, sub: S) -> MediaType
        where T: Into<Cow<'static, str>>, S: Into<Cow<'static, str>>
    {
        MediaType {
            source: Source::None,
            top: Indexed::Concrete(top.into()),
            sub: Indexed::Concrete(sub.into()),
            params: MediaParams::Static(&[]),
        }
    }

    /// Sets the parameters `parameters` on `self`.
    ///
    /// # Example
    ///
    /// Create a custom `application/x-id; id=1` media type:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::MediaType;
    ///
    /// let id = MediaType::new("application", "x-id").with_params(("id", "1"));
    /// assert_eq!(id.to_string(), "application/x-id; id=1".to_string());
    /// ```
    ///
    /// Create a custom `text/person; name=bob; weight=175` media type:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::MediaType;
    ///
    /// let mt = MediaType::new("text", "person")
    ///     .with_params([("name", "bob"), ("ref", "2382")]);
    ///
    /// assert_eq!(mt.to_string(), "text/person; name=bob; ref=2382".to_string());
    /// ```
    pub fn with_params<K, V, P>(mut self, ps: P) -> MediaType
        where K: Into<Cow<'static, str>>,
              V: Into<Cow<'static, str>>,
              P: IntoCollection<(K, V)>
    {
        use Indexed::Concrete;

        let params = ps.mapped(|(k, v)| (Concrete(k.into()), Concrete(v.into())));
        self.params = MediaParams::Dynamic(params);
        self
    }

    /// A `const` variant of [`MediaType::with_params()`]. Creates a new
    /// `MediaType` with top-level type `top`, subtype `sub`, and parameters
    /// `params`, which may be empty.
    ///
    /// # Example
    ///
    /// Create a custom `application/x-person` media type:
    ///
    /// ```rust
    /// use rocket::http::MediaType;
    ///
    /// let custom = MediaType::const_new("application", "x-person", &[]);
    /// assert_eq!(custom.top(), "application");
    /// assert_eq!(custom.sub(), "x-person");
    /// ```
    #[inline]
    pub const fn const_new(
        top: &'static str,
        sub: &'static str,
        params: &'static [(&'static str, &'static str)]
    ) -> MediaType {
        MediaType {
            source: Source::None,
            top: Indexed::Concrete(Cow::Borrowed(top)),
            sub: Indexed::Concrete(Cow::Borrowed(sub)),
            params: MediaParams::Static(params),
        }
    }

    #[inline]
    pub(crate) const fn new_known(
        source: &'static str,
        top: &'static str,
        sub: &'static str,
        params: &'static [(&'static str, &'static str)]
    ) -> MediaType {
        MediaType {
            source: Source::Known(source),
            top: Indexed::Concrete(Cow::Borrowed(top)),
            sub: Indexed::Concrete(Cow::Borrowed(sub)),
            params: MediaParams::Static(params),
        }
    }

    pub(crate) fn known_source(&self) -> Option<&'static str> {
        match self.source {
            Source::Known(string) => Some(string),
            Source::Custom(Cow::Borrowed(string)) => Some(string),
            _ => None
        }
    }

    known_shorthands!(parse_flexible);

    known_extensions!(from_extension);

    /// Returns the top-level type for this media type. The return type,
    /// `UncasedStr`, has caseless equality comparison and hashing.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::MediaType;
    ///
    /// let plain = MediaType::Plain;
    /// assert_eq!(plain.top(), "text");
    /// assert_eq!(plain.top(), "TEXT");
    /// assert_eq!(plain.top(), "Text");
    /// ```
    #[inline]
    pub fn top(&self) -> &UncasedStr {
        self.top.from_source(self.source.as_str()).into()
    }

    /// Returns the subtype for this media type. The return type,
    /// `UncasedStr`, has caseless equality comparison and hashing.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::MediaType;
    ///
    /// let plain = MediaType::Plain;
    /// assert_eq!(plain.sub(), "plain");
    /// assert_eq!(plain.sub(), "PlaIN");
    /// assert_eq!(plain.sub(), "pLaIn");
    /// ```
    #[inline]
    pub fn sub(&self) -> &UncasedStr {
        self.sub.from_source(self.source.as_str()).into()
    }

    /// Returns a `u8` representing how specific the top-level type and subtype
    /// of this media type are.
    ///
    /// The return value is either `0`, `1`, or `2`, where `2` is the most
    /// specific. A `0` is returned when both the top and sublevel types are
    /// `*`. A `1` is returned when only one of the top or sublevel types is
    /// `*`, and a `2` is returned when neither the top or sublevel types are
    /// `*`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::MediaType;
    ///
    /// let mt = MediaType::Plain;
    /// assert_eq!(mt.specificity(), 2);
    ///
    /// let mt = MediaType::new("text", "*");
    /// assert_eq!(mt.specificity(), 1);
    ///
    /// let mt = MediaType::Any;
    /// assert_eq!(mt.specificity(), 0);
    /// ```
    #[inline]
    pub fn specificity(&self) -> u8 {
        (self.top() != "*") as u8 + (self.sub() != "*") as u8
    }

    /// Compares `self` with `other` and returns `true` if `self` and `other`
    /// are exactly equal to each other, including with respect to their
    /// parameters and their order.
    ///
    /// This is different from the `PartialEq` implementation in that it
    /// considers parameters. In particular, `Eq` implies `PartialEq` but
    /// `PartialEq` does not imply `Eq`. That is, if `PartialEq` returns false,
    /// this function is guaranteed to return false. Similarly, if `exact_eq`
    /// returns `true`, `PartialEq` is guaranteed to return true. However, if
    /// `PartialEq` returns `true`, `exact_eq` function may or may not return
    /// `true`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::MediaType;
    ///
    /// let plain = MediaType::Plain;
    /// let plain2 = MediaType::new("text", "plain").with_params(("charset", "utf-8"));
    /// let just_plain = MediaType::new("text", "plain");
    ///
    /// // The `PartialEq` implementation doesn't consider parameters.
    /// assert!(plain == just_plain);
    /// assert!(just_plain == plain2);
    /// assert!(plain == plain2);
    ///
    /// // While `exact_eq` does.
    /// assert!(!plain.exact_eq(&just_plain));
    /// assert!(!plain2.exact_eq(&just_plain));
    /// assert!(plain.exact_eq(&plain2));
    /// ```
    pub fn exact_eq(&self, other: &MediaType) -> bool {
        self == other && self.params().eq(other.params())
    }

    /// Returns an iterator over the (key, value) pairs of the media type's
    /// parameter list. The iterator will be empty if the media type has no
    /// parameters.
    ///
    /// # Example
    ///
    /// The `MediaType::Plain` type has one parameter: `charset=utf-8`:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::MediaType;
    ///
    /// let plain = MediaType::Plain;
    /// let (key, val) = plain.params().next().unwrap();
    /// assert_eq!(key, "charset");
    /// assert_eq!(val, "utf-8");
    /// ```
    ///
    /// The `MediaType::PNG` type has no parameters:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::MediaType;
    ///
    /// let png = MediaType::PNG;
    /// assert_eq!(png.params().count(), 0);
    /// ```
    #[inline]
    pub fn params(&self) -> impl Iterator<Item=(&'_ UncasedStr, &'_ str)> + '_ {
        let raw = match self.params {
            MediaParams::Static(slice) => Either::Left(slice.iter().cloned()),
            MediaParams::Dynamic(ref vec) => {
                Either::Right(vec.iter().map(move |&(ref key, ref val)| {
                    let source_str = self.source.as_str();
                    (key.from_source(source_str), val.from_source(source_str))
                }))
            }
        };

        raw.map(|(k, v)| (k.into(), v))
    }

    /// Returns the first parameter with name `name`, if there is any.
    #[inline]
    pub fn param<'a>(&'a self, name: &str) -> Option<&'a str> {
        self.params()
            .filter(|(k, _)| *k == name)
            .map(|(_, v)| v)
            .next()
    }

    known_extensions!(extension);

    known_media_types!(media_types);
}

impl FromStr for MediaType {
    // Ideally we'd return a `ParseError`, but that requires a lifetime.
    type Err = String;

    #[inline]
    fn from_str(raw: &str) -> Result<MediaType, String> {
        parse_media_type(raw).map_err(|e| e.to_string())
    }
}

impl PartialEq for MediaType {
    #[inline(always)]
    fn eq(&self, other: &MediaType) -> bool {
        self.top() == other.top() && self.sub() == other.sub()
    }
}

impl Eq for MediaType {  }

impl Hash for MediaType {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.top().hash(state);
        self.sub().hash(state);
    }
}

impl fmt::Display for MediaType {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(src) = self.known_source() {
            src.fmt(f)
        } else {
            write!(f, "{}/{}", self.top(), self.sub())?;
            for (key, val) in self.params() {
                write!(f, "; {}={}", key, val)?;
            }

            Ok(())
        }
    }
}

impl Default for MediaParams {
    fn default() -> Self {
        MediaParams::Dynamic(SmallVec::new())
    }
}

impl Extend<(IndexedStr<'static>, IndexedStr<'static>)> for MediaParams {
    fn extend<T>(&mut self, iter: T)
        where T: IntoIterator<Item = (IndexedStr<'static>, IndexedStr<'static>)>
    {
        match self {
            MediaParams::Static(..) => panic!("can't add to static collection!"),
            MediaParams::Dynamic(ref mut v) => v.extend(iter)
        }
    }
}

impl Source {
    #[inline]
    fn as_str(&self) -> Option<&str> {
        match *self {
            Source::Known(s) => Some(s),
            Source::Custom(ref s) => Some(s.borrow()),
            Source::None => None
        }
    }
}
