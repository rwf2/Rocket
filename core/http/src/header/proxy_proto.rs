use std::fmt;

use uncased::{UncasedStr, AsUncased};

/// Parsed [`Config::proxy_proto_header`] value: identifies a forwarded HTTP
/// protocol (aka [X-Forwarded-Proto]).
///
/// The value of the header with name [`Config::proxy_proto_header`] is parsed
/// case-insensitively into this `enum`. For a given request, the parsed value,
/// if the header was present, can be retrieved via [`Request::proxy_proto()`]
/// or directly as a [request guard]. That value is used to determine whether a
/// request's context is likely secure ([`Request::context_is_likely_secure()`])
/// which in-turn is used to determine whether the `Secure` cookie flag is set
/// by default when [cookies are added] to a `CookieJar`.
///
/// [X-Forwarded-Proto]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-Forwarded-Proto
/// [`Config::proxy_proto_header`]: ../../rocket/struct.Config.html#structfield.proxy_proto_header
/// [`Request::proxy_proto()`]: ../../rocket/request/struct.Request.html#method.proxy_proto
/// [`Request::context_is_likely_secure()`]: ../../rocket/request/struct.Request.html#method.context_is_likely_secure
/// [cookies are added]: ../..//rocket/http/struct.CookieJar.html#method.add
/// [request guard]: ../../rocket/request/trait.FromRequest.html#provided-implementations
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProxyProto<'a> {
    /// `"http"`: Hypertext Transfer Protocol.
    Http,
    /// `"https"`: Hypertext Transfer Protocol Secure.
    Https,
    /// Any protocol name other than `"http"` or `"https"`.
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
