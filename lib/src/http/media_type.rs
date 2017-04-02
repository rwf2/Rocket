use std::borrow::Cow;
use std::str::FromStr;
use std::fmt;
use std::hash::{Hash, Hasher};

use http::IntoCollection;
use http::uncased::{uncased_eq, UncasedStr};
use http::parse::{IndexedStr, parse_media_type};

use smallvec::SmallVec;

#[derive(Debug, Clone)]
struct MediaParam {
    key: IndexedStr,
    value: IndexedStr,
}

// FIXME: `Static` is needed for `const` items. Need `const SmallVec::new`.
#[derive(Debug, Clone)]
pub enum MediaParams {
    Static(&'static [(IndexedStr, IndexedStr)]),
    Dynamic(SmallVec<[(IndexedStr, IndexedStr); 2]>)
}

// Describe a media type. In particular, describe its comparison and hashing
// semantics.
#[derive(Debug, Clone)]
pub struct MediaType {
    /// Storage for the entire media type string. This will be `Some` when the
    /// media type was parsed from a string and `None` when it was created
    /// manually.
    #[doc(hidden)]
    pub source: Option<Cow<'static, str>>,
    /// The top-level type.
    #[doc(hidden)]
    pub top: IndexedStr,
    /// The subtype.
    #[doc(hidden)]
    pub sub: IndexedStr,
    /// The parameters, if any.
    #[doc(hidden)]
    pub params: MediaParams
}

macro_rules! media_str {
    ($string:expr) => (IndexedStr::Concrete(Cow::Borrowed($string)))
}

macro_rules! media_types {
    ($($name:ident ($check:ident): $str:expr, $t:expr,
        $s:expr $(; $k:expr => $v:expr)*),+) => {
        $(
            #[doc="Media type for <b>"] #[doc=$str] #[doc="</b>: <i>"]
            #[doc=$t] #[doc="/"] #[doc=$s]
            $(#[doc="; "] #[doc=$k] #[doc=" = "] #[doc=$v])*
            #[doc="</i>"]
            #[allow(non_upper_case_globals)]
            pub const $name: MediaType = MediaType {
                source: None,
                top: media_str!($t),
                sub: media_str!($s),
                params: MediaParams::Static(&[$((media_str!($k), media_str!($v))),*])
            };

            #[doc="Returns `true` if `self` is the media type for <b>"]
            #[doc=$str]
            #[doc="</b>, "]
            /// without considering parameters.
            #[inline(always)]
            pub fn $check(&self) -> bool {
                *self == MediaType::$name
            }
         )+

        /// Returns `true` if this MediaType is known to Rocket, that is,
        /// there is an associated constant for `self`.
        pub fn is_known(&self) -> bool {
            $(if self.$check() { return true })+
            false
        }
    };
}

macro_rules! from_extension {
    ($($ext:expr => $name:ident),*) => (
        /// Returns the Media-Type associated with the extension `ext`. Not all
        /// extensions are recognized. If an extensions is not recognized, then this
        /// method returns a ContentType of `Any`. The currently recognized
        /// extensions include
        $(#[doc=$ext]#[doc=","])*
        /// and is likely to grow.
        ///
        /// # Example
        ///
        /// A recognized content type:
        ///
        /// ```rust
        /// use rocket::http::ContentType;
        ///
        /// let xml = ContentType::from_extension("xml");
        /// assert!(xml.is_xml());
        /// ```
        ///
        /// An unrecognized content type:
        ///
        /// ```rust
        /// use rocket::http::ContentType;
        ///
        /// let foo = ContentType::from_extension("foo");
        /// assert!(foo.is_any());
        /// ```
        pub fn from_extension(ext: &str) -> Option<MediaType> {
            match ext {
                $(x if uncased_eq(x, $ext) => Some(MediaType::$name)),*,
                _ => None
            }
        }
    )
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
            source: None,
            top: IndexedStr::Concrete(top.into()),
            sub: IndexedStr::Concrete(sub.into()),
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
    /// use rocket::http::MediaType;
    ///
    /// let id = MediaType::with_params("application", "x-id", ("id", "1"));
    /// assert_eq!(id.to_string(), "application/x-id; id=1".to_string());
    /// ```
    ///
    /// Create a custom `text/person; name=bob; weight=175` media type:
    ///
    /// ```rust
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
            IndexedStr::Concrete(key.into()),
            IndexedStr::Concrete(val.into())
        ));


        MediaType {
            source: None,
            top: IndexedStr::Concrete(top.into()),
            sub: IndexedStr::Concrete(sub.into()),
            params: MediaParams::Dynamic(params)
        }
    }

    known_extensions!(from_extension);

    /// Returns the top-level type for this media type. The return type,
    /// `UncasedStr`, has caseless equality comparison and hashing.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::http::MediaType;
    ///
    /// let plain = MediaType::Plain;
    /// assert_eq!(plain.top(), "text");
    /// assert_eq!(plain.top(), "TEXT");
    /// assert_eq!(plain.top(), "Text");
    /// ```
    #[inline]
    pub fn top(&self) -> &UncasedStr {
        self.top.to_str(self.source.as_ref()).into()
    }

    /// Returns the subtype for this media type. The return type,
    /// `UncasedStr`, has caseless equality comparison and hashing.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::http::MediaType;
    ///
    /// let plain = MediaType::Plain;
    /// assert_eq!(plain.sub(), "plain");
    /// assert_eq!(plain.sub(), "PlaIN");
    /// assert_eq!(plain.sub(), "pLaIn");
    /// ```
    #[inline]
    pub fn sub(&self) -> &UncasedStr {
        self.sub.to_str(self.source.as_ref()).into()
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
    /// are exactly equal to eachother, including with respect to their
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
                let source_str = self.source.as_ref();
                (key.to_str(source_str), val.to_str(source_str))
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
        write!(f, "{}/{}", self.top(), self.sub())?;
        for (key, val) in self.params() {
            write!(f, "; {}={}", key, val)?;
        }

        Ok(())
    }
}
