use std::fmt;
use std::str::FromStr;

use self::Method::*;

use crate::hyper;

// TODO: Support non-standard methods, here and in codegen?

/// Representation of HTTP methods.
///
/// # (De)serialization
///
/// `Method` is both `Serialize` and `Deserialize`, represented as an
/// [uncased](crate::uncased) string. For example, [`Method::Get`] serializes to
/// `"GET"` and deserializes from any casing of `"GET"` including `"get"`,
/// `"GeT"`, and `"GET"`.
///
/// ```rust
/// # #[cfg(feature = "serde")] mod serde {
/// # use serde_ as serde;
/// use serde::{Serialize, Deserialize};
/// use rocket::http::Method;
///
/// #[derive(Deserialize, Serialize)]
/// # #[serde(crate = "serde_")]
/// struct Foo {
///     method: Method,
/// }
/// # }
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Method {
    /// The `GET` variant.
    Get,
    /// The `PUT` variant.
    Put,
    /// The `POST` variant.
    Post,
    /// The `DELETE` variant.
    Delete,
    /// The `OPTIONS` variant.
    Options,
    /// The `HEAD` variant.
    Head,
    /// The `TRACE` variant.
    Trace,
    /// The `CONNECT` variant.
    Connect,
    /// The `PATCH` variant.
    Patch
}

impl Method {
    /// WARNING: This is unstable! Do not use this method outside of Rocket!
    #[doc(hidden)]
    pub fn from_hyp(method: &hyper::Method) -> Option<Method> {
        match *method {
            hyper::Method::GET => Some(Get),
            hyper::Method::PUT => Some(Put),
            hyper::Method::POST => Some(Post),
            hyper::Method::DELETE => Some(Delete),
            hyper::Method::OPTIONS => Some(Options),
            hyper::Method::HEAD => Some(Head),
            hyper::Method::TRACE => Some(Trace),
            hyper::Method::CONNECT => Some(Connect),
            hyper::Method::PATCH => Some(Patch),
            _ => None,
        }
    }

    /// Returns `true` if an HTTP request with the method represented by `self`
    /// always supports a payload.
    ///
    /// The following methods always support payloads:
    ///
    ///   * `PUT`, `POST`, `DELETE`, `PATCH`
    ///
    /// The following methods _do not_ always support payloads:
    ///
    ///   * `GET`, `HEAD`, `CONNECT`, `TRACE`, `OPTIONS`
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::Method;
    ///
    /// assert_eq!(Method::Get.supports_payload(), false);
    /// assert_eq!(Method::Post.supports_payload(), true);
    /// ```
    #[inline]
    pub fn supports_payload(self) -> bool {
        match self {
            Put | Post | Delete | Patch => true,
            Get | Head | Connect | Trace | Options => false,
        }
    }

    /// Returns the string representation of `self`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::Method;
    ///
    /// assert_eq!(Method::Get.as_str(), "GET");
    /// ```
    #[inline]
    pub fn as_str(self) -> &'static str {
        match self {
            Get => "GET",
            Put => "PUT",
            Post => "POST",
            Delete => "DELETE",
            Options => "OPTIONS",
            Head => "HEAD",
            Trace => "TRACE",
            Connect => "CONNECT",
            Patch => "PATCH",
        }
    }
}

impl FromStr for Method {
    type Err = ();

    // According to the RFC, method names are case-sensitive. But some old
    // clients don't follow this, so we just do a case-insensitive match here.
    fn from_str(s: &str) -> Result<Method, ()> {
        match s {
            x if uncased::eq(x, Get.as_str()) => Ok(Get),
            x if uncased::eq(x, Put.as_str()) => Ok(Put),
            x if uncased::eq(x, Post.as_str()) => Ok(Post),
            x if uncased::eq(x, Delete.as_str()) => Ok(Delete),
            x if uncased::eq(x, Options.as_str()) => Ok(Options),
            x if uncased::eq(x, Head.as_str()) => Ok(Head),
            x if uncased::eq(x, Trace.as_str()) => Ok(Trace),
            x if uncased::eq(x, Connect.as_str()) => Ok(Connect),
            x if uncased::eq(x, Patch.as_str()) => Ok(Patch),
            _ => Err(()),
        }
    }
}

impl fmt::Display for Method {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

#[cfg(feature = "serde")]
mod serde {
    use std::fmt;
    use super::*;

    use serde_::ser::{Serialize, Serializer};
    use serde_::de::{Deserialize, Deserializer, Error, Visitor, Unexpected};

    impl<'a> Serialize for Method {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.serialize_str(self.as_str())
        }
    }

    struct DeVisitor;

    impl<'de> Visitor<'de> for DeVisitor {
        type Value = Method;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "valid HTTP method string")
        }

        fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
            Method::from_str(v).map_err(|_| E::invalid_value(Unexpected::Str(v), &self))
        }
    }

    impl<'de> Deserialize<'de> for Method {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            deserializer.deserialize_str(DeVisitor)
        }
    }
}
