use core::f32;
use std::borrow::Cow;
use std::str::FromStr;
use std::fmt;
use std::hash::{Hash, Hasher};

use crate::uncased::UncasedStr;
use crate::parse::{Indexed, IndexedStr, parse_content_coding};
use crate::Source;

/// An HTTP content coding.
///
/// # Usage
///
/// A `ContentCoding` should rarely be used directly. Instead, one is typically used
/// indirectly via types like [`Accept-Encoding`](crate::Accept-Encoding) and
/// [`ContentEncoding`](crate::ContentEncoding), which internally contain `ContentCoding`s.
/// Nonetheless, a `ContentCoding` can be created via the [`ContentCoding::new()`]
/// and [`ContentCoding::with_weight()`].
/// The preferred method, however, is to create a `ContentCoding` via an associated
/// constant.
///
/// ## Example
///
/// A content coding of `gzip` can be instantiated via the
/// [`ContentCoding::GZIP`] constant:
///
/// ```rust
/// # extern crate rocket;
/// use rocket::http::ContentCoding;
///
/// let gzip = ContentCoding::GZIP;
/// assert_eq!(gzip.coding(), "gzip");
///
/// let gzip = ContentCoding::new("gzip");
/// assert_eq!(ContentCoding::GZIP, gzip);
/// ```
///
/// # Comparison and Hashing
///
/// The `PartialEq` and `Hash` implementations for `ContentCoding` _do not_ take
/// into account parameters. This means that a content coding of `gzip` is
/// equal to a content coding of `gzip; q=1`, for instance. This is
/// typically the comparison that is desired.
///
/// If an exact comparison is desired that takes into account parameters, the
/// [`exact_eq()`](ContentCoding::exact_eq()) method can be used.
#[derive(Debug, Clone)]
pub struct ContentCoding {
    /// InitCell for the entire content codding string.
    pub(crate) source: Source,
    /// The top-level type.
    pub(crate) coding: IndexedStr<'static>,
    /// The parameters, if any.
    pub(crate) weight: Option<f32>,
}

macro_rules! content_codings {
    // ($($name:ident ($check:ident): $str:expr, $t:expr, $s:expr $(; $k:expr => $v:expr)*,)+)
    ($($name:ident ($check:ident): $str:expr, $c:expr,)+) => {
        $(
            /// Content Coding for
            #[doc = concat!("**", $str, "**: ")]
            #[doc = concat!("`", $c, "`")]
            #[allow(non_upper_case_globals)]
            pub const $name: ContentCoding = ContentCoding::new_known(
                $c,
                $c, 
                None,
            );
        )+
        
        /// Returns `true` if this ContentCoding is known to Rocket. In other words,
        /// returns `true` if there is an associated constant for `self`.
        pub fn is_known(&self) -> bool {
            if let Source::Known(_) = self.source {
                return true;
            }
            
            $(if self.$check() { return true })+
            false
        }
        
        $(
            /// Returns `true` if the top-level and sublevel types of
            /// `self` are the same as those of
            #[doc = concat!("`ContentCoding::", stringify!($name), "`, ")]
            /// i.e
            #[doc = concat!("`", $c, "`.")]
            #[inline(always)]
            pub fn $check(&self) -> bool {
                *self == ContentCoding::$name
            }
        )+
    }
}

impl ContentCoding {
    /// Creates a new `ContentCoding` for `coding`.
    /// This should _only_ be used to construct uncommon or custom content codings.
    /// Use an associated constant for everything else.
    ///
    /// # Example
    ///
    /// Create a custom `rar` content coding:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::ContentCoding;
    ///
    /// let custom = ContentCoding::new("rar");
    /// assert_eq!(custom.coding(), "rar");
    /// ```
    #[inline]
    pub fn new<C>(coding: C) -> ContentCoding
        where C: Into<Cow<'static, str>>
    {
        ContentCoding {
            source: Source::None,
            coding: Indexed::Concrete(coding.into()),
            weight: None,
        }
    }

    /// Sets the weight `weight` on `self`.
    ///
    /// # Example
    ///
    /// Create a custom `rar; q=1` content coding:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::ContentCoding;
    ///
    /// let id = ContentCoding::new("rar").with_weight(1);
    /// assert_eq!(id.to_string(), "rar; q=1".to_string());
    /// ```
    pub fn with_weight(mut self, p: f32) -> ContentCoding
    {
        self.weight = Some(p);
        self
    }

    /// A `const` variant of [`ContentCoding::with_params()`]. Creates a new
    /// `ContentCoding` with coding `coding`, and weight
    /// `weight`, which may be empty.
    ///
    /// # Example
    ///
    /// Create a custom `rar` content coding:
    ///
    /// ```rust
    /// use rocket::http::ContentCoding;
    ///
    /// let custom = ContentCoding::const_new("rar", None);
    /// assert_eq!(custom.coding(), "rar");
    /// assert_eq!(custom.weight(), None);
    /// ```
    #[inline]
    pub const fn const_new(
        coding: &'static str,
        weight: Option<f32>,
    ) -> ContentCoding {
        ContentCoding {
            source: Source::None,
            coding: Indexed::Concrete(Cow::Borrowed(coding)),
            weight: weight,
        }
    }

    #[inline]
    pub(crate) const fn new_known(
        source: &'static str,
        coding: &'static str,
        weight: Option<f32>,
    ) -> ContentCoding {
        ContentCoding {
            source: Source::Known(source),
            coding: Indexed::Concrete(Cow::Borrowed(coding)),
            weight: weight,
        }
    }

    pub(crate) fn known_source(&self) -> Option<&'static str> {
        match self.source {
            Source::Known(string) => Some(string),
            Source::Custom(Cow::Borrowed(string)) => Some(string),
            _ => None
        }
    }

    /// Returns the coding for this ContentCoding. The return type,
    /// `UncasedStr`, has caseless equality comparison and hashing.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::ContentCoding;
    ///
    /// let gzip = ContentCoding::GZIP;
    /// assert_eq!(gzip.coding(), "gzip");
    /// assert_eq!(gzip.top(), "GZIP");
    /// assert_eq!(gzip.top(), "Gzip");
    /// ```
    #[inline]
    pub fn coding(&self) -> &UncasedStr {
        self.coding.from_source(self.source.as_str()).into()
    }

    /// Compares `self` with `other` and returns `true` if `self` and `other`
    /// are exactly equal to each other, including with respect to their
    /// weight.
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
    /// use rocket::http::ContentCoding;
    ///
    /// let gzip = ContentCoding::GZIP;
    /// let gzip2 = ContentCoding::new("gzip").with_weight(1);
    /// let just_plain = ContentCoding::new("gzip");
    ///
    /// // The `PartialEq` implementation doesn't consider parameters.
    /// assert!(plain == just_plain);
    /// assert!(just_plain == plain2);
    /// assert!(plain == plain2);
    ///
    /// // While `exact_eq` does.
    /// assert!(plain.exact_eq(&just_plain));
    /// assert!(!plain2.exact_eq(&just_plain));
    /// assert!(!plain.exact_eq(&plain2));
    /// ```
    pub fn exact_eq(&self, other: &ContentCoding) -> bool {
        self == other && self.weight().eq(other.weight())
    }

    /// Returns the weight content coding.
    ///
    /// # Example
    ///
    /// The `ContentCoding::GZIP` type has no specified weight:
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::ContentCoding;
    ///
    /// let gzip = ContentCoding::GZIP;
    /// let weight = gzip.weight();
    /// assert_eq!(weight, None);
    /// ```
    #[inline]
    pub fn weight(&self) -> &Option<f32> {
        &self.weight
    }

    known_content_codings!(content_codings);
}

impl FromStr for ContentCoding {
    // Ideally we'd return a `ParseError`, but that requires a lifetime.
    type Err = String;

    #[inline]
    fn from_str(raw: &str) -> Result<ContentCoding, String> {
        parse_content_coding(raw).map_err(|e| e.to_string())
    }
}

impl PartialEq for ContentCoding {
    #[inline(always)]
    fn eq(&self, other: &ContentCoding) -> bool {
        self.coding() == other.coding()
    }
}

impl Eq for ContentCoding {  }

impl Hash for ContentCoding {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.coding().hash(state);
    }
}

impl fmt::Display for ContentCoding {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(src) = self.known_source() {
            src.fmt(f)
        } else {
            write!(f, "{}", self.coding())?;
            if let Some(weight) = self.weight() {
                write!(f, "; q={}", weight)?;
            }

            Ok(())
        }
    }
}
