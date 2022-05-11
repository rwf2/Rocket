use std::fmt::{self, Display};
use std::borrow::Cow;

use crate::ext::IntoOwned;
use crate::uri::{Origin, Authority, Absolute, Reference, Asterisk};
use crate::uri::error::{Error, TryFromUriError};

/// An `enum` encapsulating any of the possible URI variants.
///
/// # Usage
///
/// In Rocket, this type will rarely be used directly. Instead, you will
/// typically encounter URIs via the [`Origin`] type. This is because all
/// incoming requests accepred by Rocket contain URIs in origin-form.
///
/// ## Parsing
///
/// The `Uri` type implements a full, zero-allocation, zero-copy [RFC 7230]
/// compliant "request target" parser with limited liberties for real-world
/// deviations. In particular, the parser deviates as follows:
///
///   * It accepts `%` characters without two trailing hex-digits.
///
///   * It accepts the following additional unencoded characters in query parts,
///     to match real-world browser behavior:
///
///     `{`, `}`, `[`,  `]`, `\`,  `^`,  <code>&#96;</code>, `|`
///
/// To parse an `&str` into a `Uri`, use [`Uri::parse()`]. Alternatively, you
/// may also use the `TryFrom<&str>` and `TryFrom<String>` trait implementation.
/// To inspect the parsed type, match on the resulting `enum` and use the
/// methods of the internal structure.
///
/// [RFC 7230]: https://tools.ietf.org/html/rfc7230
#[derive(Debug, PartialEq, Clone)]
pub enum Uri<'a> {
    /// An asterisk: exactly `*`.
    Asterisk(Asterisk),
    /// An origin URI.
    Origin(Origin<'a>),
    /// An authority URI.
    Authority(Authority<'a>),
    /// An absolute URI.
    Absolute(Absolute<'a>),
    /// A URI reference.
    Reference(Reference<'a>),
}

impl<'a> Uri<'a> {
    /// Parses the string `string` into a `Uri` of kind `T`.
    ///
    /// This is identical to `T::parse(string).map(Uri::from)`.
    ///
    /// `T` is typically one of [`Asterisk`], [`Origin`], [`Authority`],
    /// [`Absolute`], or [`Reference`]. Parsing never allocates. Returns an
    /// `Error` if `string` is not a valid URI of kind `T`.
    ///
    /// To perform an ambgiuous parse into _any_ valid URI type, use
    /// [`Uri::parse_any()`].
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::uri::{Uri, Origin};
    ///
    /// // Parse a valid origin URI (note: in practice, use `Origin::parse()`).
    /// let uri = Uri::parse::<Origin>("/a/b/c?query").expect("valid URI");
    /// let origin = uri.origin().expect("origin URI");
    /// assert_eq!(origin.path(), "/a/b/c");
    /// assert_eq!(origin.query().unwrap(), "query");
    ///
    /// // Prefer to use the `uri!()` macro for static inputs. The return value
    /// // is of the specific type, not `Uri`.
    /// let origin = uri!("/a/b/c?query");
    /// assert_eq!(origin.path(), "/a/b/c");
    /// assert_eq!(origin.query().unwrap(), "query");
    ///
    /// // Invalid URIs fail to parse.
    /// Uri::parse::<Origin>("foo bar").expect_err("invalid URI");
    /// ```
    pub fn parse<T>(string: &'a str) -> Result<Uri<'a>, Error<'_>>
        where T: Into<Uri<'a>> + TryFrom<&'a str, Error = Error<'a>>
    {
        T::try_from(string).map(|v| v.into())
    }

    /// Parse `string` into a the "best fit" URI type.
    ///
    /// Always prefer to use `uri!()` for statically known inputs.
    ///
    /// Because URI parsing is ambgious (that is, there isn't a one-to-one
    /// mapping between strings and a URI type), the internal type returned by
    /// this method _may_ not be the desired type. This method chooses the "best
    /// fit" type for a given string by preferring to parse in the following
    /// order:
    ///
    ///   * `Asterisk`
    ///   * `Origin`
    ///   * `Authority`
    ///   * `Absolute`
    ///   * `Reference`
    ///
    /// Thus, even though `*` is a valid `Asterisk` _and_ `Reference` URI, it
    /// will parse as an `Asterisk`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::uri::{Uri, Origin, Reference};
    ///
    /// // An absolute path is an origin _unless_ it contains a fragment.
    /// let uri = Uri::parse_any("/a/b/c?query").expect("valid URI");
    /// let origin = uri.origin().expect("origin URI");
    /// assert_eq!(origin.path(), "/a/b/c");
    /// assert_eq!(origin.query().unwrap(), "query");
    ///
    /// let uri = Uri::parse_any("/a/b/c?query#fragment").expect("valid URI");
    /// let reference = uri.reference().expect("reference URI");
    /// assert_eq!(reference.path(), "/a/b/c");
    /// assert_eq!(reference.query().unwrap(), "query");
    /// assert_eq!(reference.fragment().unwrap(), "fragment");
    ///
    /// // Prefer to use the `uri!()` macro for static inputs. The return type
    /// // is the internal type, not `Uri`. The explicit type is not required.
    /// let uri: Origin = uri!("/a/b/c?query");
    /// let uri: Reference = uri!("/a/b/c?query#fragment");
    /// ```
    pub fn parse_any(string: &'a str) -> Result<Uri<'a>, Error<'_>> {
        crate::parse::uri::from_str(string)
    }

    /// Returns the internal instance of `Origin` if `self` is a `Uri::Origin`.
    /// Otherwise, returns `None`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::uri::{Uri, Absolute, Origin};
    ///
    /// let uri = Uri::parse::<Origin>("/a/b/c?query").expect("valid URI");
    /// assert!(uri.origin().is_some());
    ///
    /// let uri = Uri::from(uri!("/a/b/c?query"));
    /// assert!(uri.origin().is_some());
    ///
    /// let uri = Uri::parse::<Absolute>("https://rocket.rs").expect("valid URI");
    /// assert!(uri.origin().is_none());
    ///
    /// let uri = Uri::from(uri!("https://rocket.rs"));
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
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::uri::{Uri, Absolute, Authority};
    ///
    /// let uri = Uri::parse::<Authority>("user:pass@domain.com").expect("valid URI");
    /// assert!(uri.authority().is_some());
    ///
    /// let uri = Uri::from(uri!("user:pass@domain.com"));
    /// assert!(uri.authority().is_some());
    ///
    /// let uri = Uri::parse::<Absolute>("https://rocket.rs").expect("valid URI");
    /// assert!(uri.authority().is_none());
    ///
    /// let uri = Uri::from(uri!("https://rocket.rs"));
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
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::uri::{Uri, Absolute, Origin};
    ///
    /// let uri = Uri::parse::<Absolute>("http://rocket.rs").expect("valid URI");
    /// assert!(uri.absolute().is_some());
    ///
    /// let uri = Uri::from(uri!("http://rocket.rs"));
    /// assert!(uri.absolute().is_some());
    ///
    /// let uri = Uri::parse::<Origin>("/path").expect("valid URI");
    /// assert!(uri.absolute().is_none());
    ///
    /// let uri = Uri::from(uri!("/path"));
    /// assert!(uri.absolute().is_none());
    /// ```
    pub fn absolute(&self) -> Option<&Absolute<'a>> {
        match self {
            Uri::Absolute(ref inner) => Some(inner),
            _ => None
        }
    }

    /// Returns the internal instance of `Reference` if `self` is a
    /// `Uri::Reference`. Otherwise, returns `None`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::uri::{Uri, Absolute, Reference};
    ///
    /// let uri = Uri::parse::<Reference>("foo/bar").expect("valid URI");
    /// assert!(uri.reference().is_some());
    ///
    /// let uri = Uri::from(uri!("foo/bar"));
    /// assert!(uri.reference().is_some());
    ///
    /// let uri = Uri::parse::<Absolute>("https://rocket.rs").expect("valid URI");
    /// assert!(uri.reference().is_none());
    ///
    /// let uri = Uri::from(uri!("https://rocket.rs"));
    /// assert!(uri.reference().is_none());
    /// ```
    pub fn reference(&self) -> Option<&Reference<'a>> {
        match self {
            Uri::Reference(ref inner) => Some(inner),
            _ => None
        }
    }
}

pub(crate) unsafe fn as_utf8_unchecked(input: Cow<'_, [u8]>) -> Cow<'_, str> {
    match input {
        Cow::Borrowed(bytes) => Cow::Borrowed(std::str::from_utf8_unchecked(bytes)),
        Cow::Owned(bytes) => Cow::Owned(String::from_utf8_unchecked(bytes))
    }
}

// impl<'a> TryFrom<&'a str> for Uri<'a> {
//     type Error = Error<'a>;
//
//     #[inline]
//     fn try_from(string: &'a str) -> Result<Uri<'a>, Self::Error> {
//         Uri::parse(string)
//     }
// }
//
// impl TryFrom<String> for Uri<'static> {
//     type Error = Error<'static>;
//
//     #[inline]
//     fn try_from(string: String) -> Result<Uri<'static>, Self::Error> {
//         // TODO: Potentially optimize this like `Origin::parse_owned`.
//         Uri::parse(&string)
//             .map(|u| u.into_owned())
//             .map_err(|e| e.into_owned())
//     }
// }

impl IntoOwned for Uri<'_> {
    type Owned = Uri<'static>;

    fn into_owned(self) -> Uri<'static> {
        match self {
            Uri::Origin(origin) => Uri::Origin(origin.into_owned()),
            Uri::Authority(authority) => Uri::Authority(authority.into_owned()),
            Uri::Absolute(absolute) => Uri::Absolute(absolute.into_owned()),
            Uri::Reference(reference) => Uri::Reference(reference.into_owned()),
            Uri::Asterisk(asterisk) => Uri::Asterisk(asterisk)
        }
    }
}

impl Display for Uri<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Uri::Origin(ref origin) => write!(f, "{}", origin),
            Uri::Authority(ref authority) => write!(f, "{}", authority),
            Uri::Absolute(ref absolute) => write!(f, "{}", absolute),
            Uri::Reference(ref reference) => write!(f, "{}", reference),
            Uri::Asterisk(ref asterisk) => write!(f, "{}", asterisk)
        }
    }
}

macro_rules! impl_uri_from {
    ($T:ident $(<$lt:lifetime>)?) => (
        impl<'a> From<$T $(<$lt>)?> for Uri<'a> {
            fn from(other: $T $(<$lt>)?) -> Uri<'a> {
                Uri::$T(other)
            }
        }

        impl<'a> TryFrom<Uri<'a>> for $T $(<$lt>)? {
            type Error = TryFromUriError;

            fn try_from(uri: Uri<'a>) -> Result<Self, Self::Error> {
                match uri {
                    Uri::$T(inner) => Ok(inner),
                    _ => Err(TryFromUriError(()))
                }
            }
        }

        impl<'b, $($lt)?> PartialEq<$T $(<$lt>)?> for Uri<'b> {
            fn eq(&self, other: &$T $(<$lt>)?) -> bool {
                match self {
                    Uri::$T(inner) => inner == other,
                    _ => false
                }
            }
        }

        impl<'b, $($lt)?> PartialEq<Uri<'b>> for $T $(<$lt>)? {
            fn eq(&self, other: &Uri<'b>) -> bool {
                match other {
                    Uri::$T(inner) => inner == self,
                    _ => false
                }
            }
        }
    )
}

impl_uri_from!(Origin<'a>);
impl_uri_from!(Authority<'a>);
impl_uri_from!(Absolute<'a>);
impl_uri_from!(Reference<'a>);
impl_uri_from!(Asterisk);

/// Implements Serialize and Deserialize for any 'URI' looking type.
macro_rules! impl_serde {
    ($T:ty, $expected:literal) => {
        #[cfg(feature = "serde")]
        mod serde {
            use std::fmt;
            use std::marker::PhantomData;
            use super::*;

            use serde_::ser::{Serialize, Serializer};
            use serde_::de::{Deserialize, Deserializer, Error, Visitor};

            impl<'a> Serialize for $T {
                fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                    serializer.serialize_str(&self.to_string())
                }
            }

            struct DeVisitor<'a>(PhantomData<&'a $T>);

            impl<'de, 'a> Visitor<'de> for DeVisitor<'a> {
                type Value = $T;

                fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                    write!(formatter, $expected)
                }

                fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
                    <$T>::parse_owned(v.to_string()).map_err(Error::custom)
                }

                fn visit_string<E: Error>(self, v: String) -> Result<Self::Value, E> {
                    <$T>::parse_owned(v).map_err(Error::custom)
                }
            }

            impl<'a, 'de> Deserialize<'de> for $T {
                fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                    deserializer.deserialize_str(DeVisitor(PhantomData))
                }
            }
        }
    };
}

/// Implements traits from `impl_base_traits` and IntoOwned for a URI.
macro_rules! impl_traits {
    ($T:ident, $($field:ident),* $(,)?) => {
        impl_base_traits!($T, $($field),*);

        impl crate::ext::IntoOwned for $T<'_> {
            type Owned = $T<'static>;

            fn into_owned(self) -> $T<'static> {
                $T {
                    source: self.source.into_owned(),
                    $($field: self.$field.into_owned()),*
                }
            }
        }
    }
}

/// Implements PartialEq, Eq, Hash, and TryFrom.
macro_rules! impl_base_traits {
    ($T:ident, $($field:ident),* $(,)?) => {
        impl std::convert::TryFrom<String> for $T<'static> {
            type Error = Error<'static>;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                $T::parse_owned(value)
            }
        }

        // Because inference doesn't take `&String` to `&str`.
        impl<'a> std::convert::TryFrom<&'a String> for $T<'a> {
            type Error = Error<'a>;

            fn try_from(value: &'a String) -> Result<Self, Self::Error> {
                $T::parse(value.as_str())
            }
        }

        impl<'a> std::convert::TryFrom<&'a str> for $T<'a> {
            type Error = Error<'a>;

            fn try_from(value: &'a str) -> Result<Self, Self::Error> {
                $T::parse(value)
            }
        }

        impl<'a, 'b> PartialEq<$T<'b>> for $T<'a> {
            fn eq(&self, other: &$T<'b>) -> bool {
                true $(&& self.$field() == other.$field())*
            }
        }

        impl PartialEq<str> for $T<'_> {
            fn eq(&self, string: &str) -> bool {
                $T::parse(string).map_or(false, |v| &v == self)
            }
        }

        impl PartialEq<&str> for $T<'_> {
            fn eq(&self, other: &&str) -> bool {
                self.eq(*other)
            }
        }

        impl PartialEq<$T<'_>> for str {
            fn eq(&self, other: &$T<'_>) -> bool {
                other.eq(self)
            }
        }

        impl Eq for $T<'_> { }

        impl std::hash::Hash for $T<'_> {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                $(self.$field().hash(state);)*
            }
        }
    }
}
