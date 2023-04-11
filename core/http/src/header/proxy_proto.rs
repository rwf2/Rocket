use std::fmt;

use uncased::{UncasedStr, AsUncased};

/// A protocol used to identify a specific protocol forwarded by an HTTP proxy.
/// Value are case-insensitive.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProxyProto<'a> {
    /// `http` value, Hypertext Transfer Protocol.
    Http,
    /// `https` value, Hypertext Transfer Protocol Secure.
    Https,
    /// Any protocol name other than `http` or `https`.
    Unknown(&'a UncasedStr),
}

impl ProxyProto<'_> {
    /// Returns `true` if `self` is `ProxyProto::Https` and `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::http::ProxyProto;
    ///
    /// assert!(ProxyProto::Https.is_https());
    /// assert!(!ProxyProto::Http.is_https());
    /// ```
    pub fn is_https(&self) -> bool {
        self == &ProxyProto::Https
    }
}

impl<'a> From<&'a str> for ProxyProto<'a> {
    fn from(value: &'a str) -> ProxyProto<'a> {
        match value.as_uncased() {
            v if v == "http" => ProxyProto::Http,
            v if v == "https" => ProxyProto::Https,
            v => ProxyProto::Unknown(v)
        }
    }
}

impl fmt::Display for ProxyProto<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match *self {
            ProxyProto::Http => "http",
            ProxyProto::Https => "https",
            ProxyProto::Unknown(s) => s.as_str(),
        })
    }
}
