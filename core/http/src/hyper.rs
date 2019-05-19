//! Re-exported hyper HTTP library types.
//!
//! All types that are re-exported from Hyper reside inside of this module.
//! These types will, with certainty, be removed with time, but they reside here
//! while necessary.

#[doc(hidden)] pub use hyper::{Body, Request, Response};
#[doc(hidden)] pub use hyper::body::Payload as Payload;
#[doc(hidden)] pub use hyper::error::Error;
#[doc(hidden)] pub use hyper::server::Server;
#[doc(hidden)] pub use hyper::service::{MakeService, Service};

#[doc(hidden)] pub use hyper::Chunk;
#[doc(hidden)] pub use http::header::HeaderName as HeaderName;
#[doc(hidden)] pub use http::header::HeaderValue as HeaderValue;
#[doc(hidden)] pub use http::method::Method;
#[doc(hidden)] pub use http::request::Parts;
#[doc(hidden)] pub use http::status::StatusCode;
#[doc(hidden)] pub use http::uri::Uri;

/// Type alias to `hyper::Response<'a, hyper::net::Fresh>`.
// TODO #[doc(hidden)] pub type FreshResponse<'a> = self::Response<'a, self::net::Fresh>;

/// Reexported Hyper header types.
pub mod header {
    use crate::Header;

    macro_rules! import_hyper_items {
        ($($item:ident),*) => ($(pub use hyper::header::$item;)*)
    }

    macro_rules! import_hyper_headers {
        ($($name:ident),*) => ($(
            pub use http::header::$name as $name;
        )*)
    }

//    import_hyper_items! {
//        Accept, AcceptCharset, AcceptEncoding, AcceptLanguage, AcceptRanges,
//        AccessControlAllowCredentials, AccessControlAllowHeaders,
//        AccessControlAllowMethods, AccessControlExposeHeaders,
//        AccessControlMaxAge, AccessControlRequestHeaders,
//        AccessControlRequestMethod, Allow, Authorization, Basic, Bearer,
//        CacheControl, Connection, ContentDisposition, ContentEncoding,
//        ContentLanguage, ContentLength, ContentRange, ContentType, Date, ETag,
//        EntityTag, Expires, From, Headers, Host, HttpDate, IfModifiedSince,
//        IfUnmodifiedSince, LastModified, Location, Origin, Prefer,
//        PreferenceApplied, Protocol, Quality, QualityItem, Referer,
//        StrictTransportSecurity, TransferEncoding, Upgrade, UserAgent,
//        AccessControlAllowOrigin, ByteRangeSpec, CacheDirective, Charset,
//        ConnectionOption, ContentRangeSpec, DispositionParam, DispositionType,
//        Encoding, Expect, IfMatch, IfNoneMatch, IfRange, Pragma, Preference,
//        ProtocolName, Range, RangeUnit, ReferrerPolicy, Vary, Scheme, q, qitem
//    }
//
    import_hyper_headers! {
        ACCEPT, ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_HEADERS,
        ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN,
        ACCESS_CONTROL_EXPOSE_HEADERS, ACCESS_CONTROL_MAX_AGE,
        ACCESS_CONTROL_REQUEST_HEADERS, ACCESS_CONTROL_REQUEST_METHOD, ACCEPT_CHARSET,
        ACCEPT_ENCODING, ACCEPT_LANGUAGE, ACCEPT_RANGES, ALLOW, CACHE_CONTROL,
        CONNECTION, CONTENT_DISPOSITION, CONTENT_ENCODING, CONTENT_LANGUAGE,
        CONTENT_LENGTH, CONTENT_RANGE, DATE, ETAG, EXPECT, EXPIRES, HOST, IF_MATCH,
        IF_MODIFIED_SINCE, IF_NONE_MATCH, IF_RANGE, IF_UNMODIFIED_SINCE, LAST_MODIFIED,
        LOCATION, ORIGIN, PRAGMA, RANGE, REFERER,
        REFERRER_POLICY, STRICT_TRANSPORT_SECURITY, TRANSFER_ENCODING, UPGRADE,
        USER_AGENT, VARY
    }
}
