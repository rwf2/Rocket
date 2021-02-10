//! Re-exported hyper HTTP library types and hyperx typed headers.
//!
//! All types that are re-exported from Hyper reside inside of this module.
//! These types will, with certainty, be removed with time, but they reside here
//! while necessary.

#[doc(hidden)] pub use hyper::{Body, Error, Request, Response};
#[doc(hidden)] pub use hyper::body::{Bytes, HttpBody, Sender as BodySender};
#[doc(hidden)] pub use hyper::rt::Executor;
#[doc(hidden)] pub use hyper::server::Server;
#[doc(hidden)] pub use hyper::service::{make_service_fn, service_fn, Service};

#[doc(hidden)] pub use http::header::HeaderMap;
#[doc(hidden)] pub use http::header::HeaderName as HeaderName;
#[doc(hidden)] pub use http::header::HeaderValue as HeaderValue;
#[doc(hidden)] pub use http::method::Method;
#[doc(hidden)] pub use http::request::Parts as RequestParts;
#[doc(hidden)] pub use http::response::Builder as ResponseBuilder;
#[doc(hidden)] pub use http::status::StatusCode;
#[doc(hidden)] pub use http::uri::{Uri, Parts as UriParts};

/// Reexported http header types.
pub mod header {
    use super::super::header::Header;
    pub use hyperx::header::Header as HyperxHeaderTrait;

    macro_rules! import_http_headers {
        ($($name:ident),*) => ($(
            pub use http::header::$name as $name;
        )*)
    }

    import_http_headers! {
        ACCEPT, ACCEPT_CHARSET, ACCEPT_ENCODING, ACCEPT_LANGUAGE, ACCEPT_RANGES,
        ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_HEADERS,
        ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN,
        ACCESS_CONTROL_EXPOSE_HEADERS, ACCESS_CONTROL_MAX_AGE,
        ACCESS_CONTROL_REQUEST_HEADERS, ACCESS_CONTROL_REQUEST_METHOD, ALLOW,
        AUTHORIZATION, CACHE_CONTROL, CONNECTION, CONTENT_DISPOSITION,
        CONTENT_ENCODING, CONTENT_LANGUAGE, CONTENT_LENGTH, CONTENT_LOCATION,
        CONTENT_RANGE, CONTENT_SECURITY_POLICY,
        CONTENT_SECURITY_POLICY_REPORT_ONLY, CONTENT_TYPE, DATE, ETAG, EXPECT,
        EXPIRES, FORWARDED, FROM, HOST, IF_MATCH, IF_MODIFIED_SINCE,
        IF_NONE_MATCH, IF_RANGE, IF_UNMODIFIED_SINCE, LAST_MODIFIED, LINK,
        LOCATION, ORIGIN, PRAGMA, RANGE, REFERER, REFERRER_POLICY, REFRESH,
        STRICT_TRANSPORT_SECURITY, TE, TRANSFER_ENCODING, UPGRADE, USER_AGENT,
        VARY
    }

    macro_rules! import_hyperx_items {
        ($($item:ident),*) => ($(pub use hyperx::header::$item as $item;)*)
    }

    macro_rules! import_hyperx_headers {
        ($($name:ident),*) => ($(
            impl ::std::convert::From<self::$name> for Header<'static> {
                fn from(header: self::$name) -> Header<'static> {
                    Header::new($name::header_name(), header.to_string())
                }
            }
        )*)
    }

    macro_rules! import_generic_hyperx_headers {
        ($($name:ident<$bound:ident>),*) => ($(
            impl <T1: 'static + $bound> ::std::convert::From<self::$name<T1>>
                for Header<'static> {
                fn from(header: self::$name<T1>) -> Header<'static> {
                    Header::new($name::<T1>::header_name(), header.to_string())
                }
            }
        )*)
    }

    import_hyperx_items! {
        Accept, AcceptCharset, AcceptEncoding, AcceptLanguage, AcceptRanges,
        AccessControlAllowCredentials, AccessControlAllowHeaders,
        AccessControlAllowMethods, AccessControlAllowOrigin,
        AccessControlExposeHeaders, AccessControlMaxAge,
        AccessControlRequestHeaders, AccessControlRequestMethod, Allow,
        Authorization, Basic, Bearer, ByteRangeSpec, CacheControl,
        CacheDirective, Charset, Connection, ConnectionOption,
        ContentDisposition, ContentEncoding, ContentLanguage, ContentLength,
        ContentLocation, ContentRange, ContentRangeSpec, ContentType, Cookie,
        Date, DispositionParam, DispositionType, Encoding, EntityTag, ETag,
        Expect, Expires, From, Host, HttpDate, IfMatch, IfModifiedSince,
        IfNoneMatch, IfRange, IfUnmodifiedSince, LastEventId, LastModified,
        Link, LinkValue, Location, Origin, Pragma, Prefer, Preference,
        PreferenceApplied, Protocol, ProtocolName, ProxyAuthorization, Quality,
        QualityItem, Range, RangeUnit, Referer, ReferrerPolicy, RetryAfter,
        Scheme, Server, SetCookie, StrictTransportSecurity,
        Te, TransferEncoding, Upgrade, UserAgent, Vary, Warning, q, qitem
    }

    import_hyperx_headers! {
        Accept, AcceptCharset, AcceptEncoding, AcceptLanguage, AcceptRanges,
        AccessControlAllowCredentials, AccessControlAllowHeaders,
        AccessControlAllowMethods, AccessControlAllowOrigin,
        AccessControlExposeHeaders, AccessControlMaxAge,
        AccessControlRequestHeaders, AccessControlRequestMethod, Allow,
        CacheControl, Connection, ContentDisposition, ContentEncoding,
        ContentLanguage, ContentLength, ContentLocation, ContentRange,
        ContentType, Cookie, Date, ETag, Expires, Expect, From, Host, IfMatch,
        IfModifiedSince, IfNoneMatch, IfUnmodifiedSince, IfRange, LastEventId,
        LastModified, Link, Location, Origin, Pragma, Prefer, PreferenceApplied,
        Range, Referer, ReferrerPolicy, RetryAfter, Server,
        StrictTransportSecurity, Te, TransferEncoding, Upgrade, UserAgent, Vary,
        Warning
    }
    import_generic_hyperx_headers! {
        Authorization<Scheme>,
        ProxyAuthorization<Scheme>
    }
    // Note: SetCookie is missing, since it must be formatted as separate header lines...
}

#[cfg(test)]
mod tests {
    use crate::header::HeaderMap;
    use super::header::HyperxHeaderTrait; // Needed for Accept::header_name() below?!?!

    #[test]
    fn add_typed_header() {
        use super::header::{Accept, QualityItem, q, qitem};
        let mut map = HeaderMap::new();
        map.add(Accept(vec![
            QualityItem::new("audio/*".parse().unwrap(), q(200)),
            qitem("audio/basic".parse().unwrap()),
            ]));
        assert_eq!(map.get_one(Accept::header_name()), Some("audio/*; q=0.2, audio/basic"));
    }

    #[test]
    fn add_typed_header_with_type_params() {
        use super::header::{Authorization, Basic};
        let mut map = HeaderMap::new();
        map.add(Authorization(Basic {
            username: "admin".to_owned(),
            password: Some("12345678".to_owned())}));
        assert_eq!(map.get_one("Authorization"), Some("Basic YWRtaW46MTIzNDU2Nzg="));
    }

}
