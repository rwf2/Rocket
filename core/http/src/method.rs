use std::fmt;
use std::str::FromStr;

self::define_methods! {
    // enum variant   method name         body   safe idempotent [RFC,section]
    Get               "GET"               maybe  yes  yes        [9110,9.3.1]
    Head              "HEAD"              maybe  yes  yes        [9110,9.3.2]
    Post              "POST"              yes    no   no         [9110,9.3.3]
    Put               "PUT"               yes    no   yes        [9110,9.3.4]
    Delete            "DELETE"            maybe  no   yes        [9110,9.3.5]
    Connect           "CONNECT"           maybe  no   no         [9110,9.3.6]
    Options           "OPTIONS"           maybe  yes  yes        [9110,9.3.7]
    Trace             "TRACE"             no     yes  yes        [9110,9.3.8]
    Patch             "PATCH"             yes    no   no         [5789,2]

    Acl               "ACL"               yes    no   yes        [3744,8.1]
    BaselineControl   "BASELINE-CONTROL"  yes    no   yes        [3253,12.6]
    Bind              "BIND"              yes    no   yes        [5842,4]
    CheckIn           "CHECKIN"           yes    no   yes        [3253,4.4]
    CheckOut          "CHECKOUT"          maybe  no   yes        [3253,4.3]
    Copy              "COPY"              maybe  no   yes        [4918,9.8]
    Label             "LABEL"             yes    no   yes        [3253,8.2]
    Link              "LINK"              maybe  no   yes        [2068,19.6.1.2]
    Lock              "LOCK"              yes    no   no         [4918,9.10]
    Merge             "MERGE"             yes    no   yes        [3253,11.2]
    MkActivity        "MKACTIVITY"        yes    no   yes        [3253,13.5]
    MkCalendar        "MKCALENDAR"        yes    no   yes        [4791,5.3.1][8144,2.3]
    MkCol             "MKCOL"             yes    no   yes        [4918,9.3][5689,3][8144,2.3]
    MkRedirectRef     "MKREDIRECTREF"     yes    no   yes        [4437,6]
    MkWorkspace       "MKWORKSPACE"       yes    no   yes        [3253,6.3]
    Move              "MOVE"              maybe  no   yes        [4918,9.9]
    OrderPatch        "ORDERPATCH"        yes    no   yes        [3648,7]
    PropFind          "PROPFIND"          yes    yes  yes        [4918,9.1][8144,2.1]
    PropPatch         "PROPPATCH"         yes    no   yes        [4918,9.2][8144,2.2]
    Rebind            "REBIND"            yes    no   yes        [5842,6]
    Report            "REPORT"            yes    yes  yes        [3253,3.6][8144,2.1]
    Search            "SEARCH"            yes    yes  yes        [5323,2]
    Unbind            "UNBIND"            yes    no   yes        [5842,5]
    Uncheckout        "UNCHECKOUT"        maybe  no   yes        [3253,4.5]
    Unlink            "UNLINK"            maybe  no   yes        [2068,19.6.1.3]
    Unlock            "UNLOCK"            maybe  no   yes        [4918,9.11]
    Update            "UPDATE"            yes    no   yes        [3253,7.1]
    UpdateRedirectRef "UPDATEREDIRECTREF" yes    no   yes        [4437,7]
    VersionControl    "VERSION-CONTROL"   yes    no   yes        [3253,3.5]
}

#[doc(hidden)]
#[macro_export]
macro_rules! define_methods {
    ($($V:ident $name:tt $body:ident $safe:ident $idem:ident $([$n:expr,$s:expr])+)*) => {
        /// An HTTP method.
        ///
        /// Each variant corresponds to a method in the [HTTP Method Registry].
        /// The string form of the method can be obtained via
        /// [`Method::as_str()`] and parsed via the `FromStr` or
        /// `TryFrom<&[u8]>` implementations. The parse implementations parse
        /// both the case-sensitive string form as well as a lowercase version
        /// of the string, but _not_ mixed-case versions.
        ///
        /// [HTTP Method Registry]: https://www.iana.org/assignments/http-methods/http-methods.xhtml
        ///
        /// # (De)Serialization
        ///
        /// `Method` is both `Serialize` and `Deserialize`.
        ///
        ///   - `Method` _serializes_ as the specification-defined string form
        ///   of the method, equivalent to the value returned from
        ///   [`Method::as_str()`].
        ///   - `Method` _deserializes_ from method's string form _or_ from a
        ///   lowercased string, equivalent to the `FromStr` implementation.
        ///
        /// For example, [`Method::Get`] serializes to `"GET"` and deserializes
        /// from either `"GET"` or `"get"` but not `"GeT"`.
        ///
        /// ```rust
        /// # #[cfg(feature = "serde")] mod serde_impl {
        /// use rocket::http::Method;
        /// use rocket::serde::{Serialize, Deserialize};
        ///
        /// #[derive(Deserialize, Serialize)]
        /// #[serde(crate = "rocket::serde")]
        /// struct Foo {
        ///     method: Method,
        /// }
        /// # }
        /// ```
        #[non_exhaustive]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Method {$(
            #[doc = concat!("The `", $name, "` method.")]
            #[doc = concat!("Defined in" $(,
                " [RFC", stringify!($n), " §", stringify!($s), "]",
                "(https://www.rfc-editor.org/rfc/rfc", stringify!($n), ".html",
                "#section-", stringify!($s), ")",
            )","+ ".")]
            ///
            #[doc = concat!("* safe: `", stringify!($safe), "`")]
            #[doc = concat!("* idempotent: `", stringify!($idem), "`")]
            #[doc = concat!("* request body: `", stringify!($body), "`")]
            $V
        ),*}

        macro_rules! lowercase {
            ($str:literal) => {{
                const BYTES: [u8; $str.len()] = {
                    let mut i = 0;
                    let _: &str = $str;
                    let mut result = [0; $str.len()];
                    while i < $str.len() {
                        result[i] = $str.as_bytes()[i].to_ascii_lowercase();
                        i += 1;
                    }

                    result
                };

                unsafe { std::str::from_utf8_unchecked(&BYTES) }
            }};
        }

        #[allow(non_upper_case_globals)]
        impl Method {
            /// A slice containing every defined method string.
            #[doc(hidden)]
            pub const ALL: &'static [&'static str] = &[$($name),*];

            /// Whether the method is considered "safe".
            ///
            /// From [RFC9110 §9.2.1](https://www.rfc-editor.org/rfc/rfc9110#section-9.2.1):
            ///
            /// > Request methods are considered "safe" if their defined
            /// semantics are essentially read-only; i.e., the client does not
            /// request, and does not expect, any state change on the origin server
            /// as a result of applying a safe method to a target resource.
            /// Likewise, reasonable use of a safe method is not expected to cause
            /// any harm, loss of property, or unusual burden on the origin server.
            /// Of the request methods defined by this specification, the GET,
            /// HEAD, OPTIONS, and TRACE methods are defined to be safe.
            ///
            /// # Example
            ///
            /// ```rust
            /// use rocket::http::Method;
            ///
            /// assert!(Method::Get.is_safe());
            /// assert!(Method::Head.is_safe());
            ///
            /// assert!(!Method::Put.is_safe());
            /// assert!(!Method::Post.is_safe());
            /// ```
            pub const fn is_safe(&self) -> bool {
                const yes: bool = true;
                const no: bool = false;

                match self {
                    $(Self::$V => $safe),*
                }
            }

            /// Whether the method is considered "idempotent".
            ///
            /// From [RFC9110 §9.2.2](https://www.rfc-editor.org/rfc/rfc9110#section-9.2.2):
            ///
            /// > A request method is considered "idempotent" if the intended
            /// effect on the server of multiple identical requests with that method
            /// is the same as the effect for a single such request. Of the request
            /// methods defined by this specification, PUT, DELETE, and safe request
            /// methods are idempotent.
            ///
            /// # Example
            ///
            /// ```rust
            /// use rocket::http::Method;
            ///
            /// assert!(Method::Get.is_idempotent());
            /// assert!(Method::Head.is_idempotent());
            /// assert!(Method::Put.is_idempotent());
            ///
            /// assert!(!Method::Post.is_idempotent());
            /// assert!(!Method::Patch.is_idempotent());
            /// ```
            pub const fn is_idempotent(&self) -> bool {
                const yes: bool = true;
                const no: bool = false;

                match self {
                    $(Self::$V => $idem),*
                }
            }

            /// Whether requests with this method are allowed to have a body.
            ///
            /// Returns:
            ///   * `Some(true)` if a request body is _always_ allowed.
            ///   * `Some(false)` if a request body is **never** allowed.
            ///   * `None` if a request body is discouraged or has no defined semantics.
            ///
            /// # Example
            ///
            /// ```rust
            /// use rocket::http::Method;
            ///
            /// assert_eq!(Method::Post.allows_request_body(), Some(true));
            /// assert_eq!(Method::Put.allows_request_body(), Some(true));
            ///
            /// assert_eq!(Method::Trace.allows_request_body(), Some(false));
            ///
            /// assert_eq!(Method::Get.allows_request_body(), None);
            /// assert_eq!(Method::Head.allows_request_body(), None);
            /// ```
            pub const fn allows_request_body(self) -> Option<bool> {
                const yes: Option<bool> = Some(true);
                const no: Option<bool> = Some(false);
                const maybe: Option<bool> = None;

                match self {
                    $(Self::$V => $body),*
                }
            }

            /// Returns the method's string representation.
            ///
            /// # Example
            ///
            /// ```rust
            /// use rocket::http::Method;
            ///
            /// assert_eq!(Method::Get.as_str(), "GET");
            /// assert_eq!(Method::Put.as_str(), "PUT");
            /// assert_eq!(Method::BaselineControl.as_str(), "BASELINE-CONTROL");
            /// ```
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$V => $name),*
                }
            }

            /// Returns a static reference to the method.
            ///
            /// # Example
            ///
            /// ```rust
            /// use rocket::http::Method;
            ///
            /// assert_eq!(Method::Get.as_ref(), &Method::Get);
            /// ```
            pub const fn as_ref(self) -> &'static Method {
                match self {
                    $(Self::$V => &Self::$V),*
                }
            }

            #[doc(hidden)]
            pub const fn variant_str(self) -> &'static str {
                match self {
                    $(Self::$V => stringify!($V)),*
                }
            }
        }

        #[cfg(test)]
        mod tests {
            use super::*;

            #[test]
            #[allow(non_upper_case_globals)]
            fn test_properties_and_parsing() {
                const yes: bool = true;
                const no: bool = false;

                $(
                    assert_eq!(Method::$V.is_idempotent(), $idem);
                    assert_eq!(Method::$V.is_safe(), $safe);
                    assert_eq!(Method::from_str($name).unwrap(), Method::$V);
                    assert_eq!(Method::from_str(lowercase!($name)).unwrap(), Method::$V);
                    assert_eq!(Method::$V.as_ref(), Method::$V);
                )*
            }
        }

        impl TryFrom<&[u8]> for Method {
            type Error = ParseMethodError;

            #[inline]
            #[allow(non_upper_case_globals)]
            fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
                mod upper { $(pub const $V: &[u8] = $name.as_bytes();)* }
                mod lower { $(pub const $V: &[u8] = lowercase!($name).as_bytes();)* }

                match value {
                    $(upper::$V | lower::$V => Ok(Self::$V),)*
                    _ => Err(ParseMethodError)
                }
            }
        }
    };
}

impl Method {
    /// Deprecated. Returns `self.allows_request_body() == Some(true)`.
    ///
    /// Use [`Method::allows_request_body()`] instead.
    #[deprecated(since = "0.6.0", note = "use Self::allows_request_body()")]
    pub const fn supports_payload(self) -> bool {
        match self.allows_request_body() {
            Some(v) => v,
            None => false,
        }
    }
}

use define_methods as define_methods;

#[derive(Debug, PartialEq, Eq)]
pub struct ParseMethodError;

impl std::error::Error for ParseMethodError { }

impl From<std::convert::Infallible> for ParseMethodError {
    fn from(infallible: std::convert::Infallible) -> Self {
        match infallible {}
    }
}

impl fmt::Display for ParseMethodError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid HTTP method")
    }
}

impl FromStr for Method {
    type Err = ParseMethodError;

    #[inline(always)]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s.as_bytes())
    }
}

impl TryFrom<&str> for Method {
    type Error = ParseMethodError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::try_from(s.as_bytes())
    }
}

impl AsRef<str> for Method {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

impl PartialEq<&Method> for Method {
    fn eq(&self, other: &&Method) -> bool {
        self == *other
    }
}

impl PartialEq<Method> for &Method {
    fn eq(&self, other: &Method) -> bool {
        *self == other
    }
}

#[cfg(feature = "serde")]
mod serde_impl {
    use super::*;

    use serde::ser::{Serialize, Serializer};
    use serde::de::{Deserialize, Deserializer, Error, Visitor, Unexpected};

    impl Serialize for Method {
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
