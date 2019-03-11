use std::borrow::{Cow, Borrow};
use std::str::FromStr;
use std::fmt;
use std::hash::{Hash, Hasher};

use ext::IntoCollection;
use uncased::{uncased_eq, UncasedStr};
use parse::{Indexed, IndexedString, parse_media_type};

use smallvec::SmallVec;

#[derive(Debug, Clone)]
struct MediaParam {
    key: IndexedString,
    value: IndexedString,
}

// FIXME: `Static` is needed for `const` items. Need `const SmallVec::new`.
#[derive(Debug, Clone)]
pub enum MediaParams {
    Static(&'static [(IndexedString, IndexedString)]),
    Dynamic(SmallVec<[(IndexedString, IndexedString); 2]>)
}

impl ::pear::parsers::Collection for MediaParams {
    type Item = (IndexedString, IndexedString);

    fn new() -> Self {
        MediaParams::Dynamic(SmallVec::new())
    }

    fn add(&mut self, item: Self::Item) {
        match *self {
            MediaParams::Static(..) => panic!("can't add to static collection!"),
            MediaParams::Dynamic(ref mut v) => v.push(item)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Known(&'static str),
    Custom(Cow<'static, str>),
    None
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

/// An HTTP media type.
///
/// # Usage
///
/// A `MediaType` should rarely be used directly. Instead, one is typically used
/// indirectly via types like [`Accept`] and [`ContentType`], which internally
/// contain `MediaType`s. Nonetheless, a `MediaType` can be created via the
/// [`MediaType::new()`], [`MediaType::with_params()`], and
/// [`MediaType::from_extension`()] methods. The preferred method, however, is
/// to create a `MediaType` via an associated constant.
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
    #[doc(hidden)]
    pub source: Source,
    /// The top-level type.
    #[doc(hidden)]
    pub top: IndexedString,
    /// The subtype.
    #[doc(hidden)]
    pub sub: IndexedString,
    /// The parameters, if any.
    #[doc(hidden)]
    pub params: MediaParams
}

macro_rules! media_str {
    ($string:expr) => (Indexed::Concrete(Cow::Borrowed($string)))
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
            pub const $name: MediaType = MediaType {
                source: Source::Known(concat!($t, "/", $s, $("; ", $k, "=", $v),*)),
                top: media_str!($t),
                sub: media_str!($s),
                params: MediaParams::Static(&[$((media_str!($k), media_str!($v))),*])
            };
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
                $(x if uncased_eq(x, $ext) => Some(MediaType::$name)),*,
                _ => None
            }
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
                $(x if uncased_eq(x, $short) => Some(MediaType::$name)),*,
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

    /// Creates a new `MediaType` with top-level type `top`, subtype `sub`, and
    /// parameters `ps`. This should _only_ be used to construct uncommon or
    /// custom media types. Use an associated constant for everything else.
    ///
    /// # Example
    ///
    /// Create a custom `application/x-id; id=1` media type:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::MediaType;
    ///
    /// let id = MediaType::with_params("application", "x-id", ("id", "1"));
    /// assert_eq!(id.to_string(), "application/x-id; id=1".to_string());
    /// ```
    ///
    /// Create a custom `text/person; name=bob; weight=175` media type:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::MediaType;
    ///
    /// let params = vec![("name", "bob"), ("ref", "2382")];
    /// let mt = MediaType::with_params("text", "person", params);
    /// assert_eq!(mt.to_string(), "text/person; name=bob; ref=2382".to_string());
    /// ```
    #[inline]
    pub fn with_params<T, S, K, V, P>(top: T, sub: S, ps: P) -> MediaType
        where T: Into<Cow<'static, str>>, S: Into<Cow<'static, str>>,
              K: Into<Cow<'static, str>>, V: Into<Cow<'static, str>>,
              P: IntoCollection<(K, V)>
    {
        let params = ps.mapped(|(key, val)| (
            Indexed::Concrete(key.into()),
            Indexed::Concrete(val.into())
        ));

        MediaType {
            source: Source::None,
            top: Indexed::Concrete(top.into()),
            sub: Indexed::Concrete(sub.into()),
            params: MediaParams::Dynamic(params)
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
    /// parameters.
    ///
    /// This is different from the `PartialEq` implementation in that it
    /// considers parameters. If `PartialEq` returns false, this function is
    /// guaranteed to return false. Similarly, if this function returns `true`,
    /// `PartialEq` is guaranteed to return true. However, if `PartialEq`
    /// returns `true`, this function may or may not return `true`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::MediaType;
    ///
    /// let plain = MediaType::Plain;
    /// let plain2 = MediaType::with_params("text", "plain", ("charset", "utf-8"));
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
        self == other && {
            let (mut a_params, mut b_params) = (self.params(), other.params());
            loop {
                match (a_params.next(), b_params.next()) {
                    (Some(a), Some(b)) if a != b => return false,
                    (Some(_), Some(_)) => continue,
                    (Some(_), None) => return false,
                    (None, Some(_)) => return false,
                    (None, None) => return true
                }
            }
        }
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
    /// let plain_params: Vec<_> = plain.params().collect();
    /// assert_eq!(plain_params, vec![("charset", "utf-8")]);
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
    pub fn params<'a>(&'a self) -> impl Iterator<Item=(&'a str, &'a str)> + 'a {
        let param_slice = match self.params {
            MediaParams::Static(slice) => slice,
            MediaParams::Dynamic(ref vec) => &vec[..],
        };

        param_slice.iter()
            .map(move |&(ref key, ref val)| {
                let source_str = self.source.as_str();
                (key.from_source(source_str), val.from_source(source_str))
            })
    }

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

impl Hash for MediaType {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.top().hash(state);
        self.sub().hash(state);

        for (key, val) in self.params() {
            key.hash(state);
            val.hash(state);
        }
    }
}

impl fmt::Display for MediaType {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Source::Known(src) = self.source {
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
