//! Re-exported hyper HTTP library types.
//!
//! All types that are re-exported from Hyper reside inside of this module.
//! These types will, with certainty, be removed with time, but they reside here
//! while necessary.

pub(crate) use hyper::server::Request as Request;
pub(crate) use hyper::server::Response as Response;
pub(crate) use hyper::server::Server as Server;
pub(crate) use hyper::server::Handler as Handler;

pub(crate) use hyper::net;

pub(crate) use hyper::method::Method;
pub(crate) use hyper::status::StatusCode;
pub(crate) use hyper::uri::RequestUri;
pub(crate) use hyper::http::h1;
pub(crate) use hyper::buffer;

pub use hyper::mime;

/// Type alias to `hyper::Response<'a, hyper::net::Fresh>`.
pub(crate) type FreshResponse<'a> = self::Response<'a, self::net::Fresh>;

/// Reexported Hyper header types.
pub mod header {
    use http::Header;

    use hyper::header::Header as HyperHeaderTrait;

    macro_rules! import_hyper_items {
        ($($item:ident),*) => ($(pub use hyper::header::$item;)*)
    }

    macro_rules! import_hyper_headers {
        ($($name:ident),*) => ($(
            impl ::std::convert::From<self::$name> for Header<'static> {
                fn from(header: self::$name) -> Header<'static> {
                    Header::new($name::header_name(), header.to_string())
                }
            }
        )*)
    }

    import_hyper_items! {
        Accept, AcceptCharset, AcceptEncoding, AcceptLanguage, AcceptRanges,
        AccessControlAllowCredentials, AccessControlAllowHeaders,
        AccessControlAllowMethods, AccessControlExposeHeaders,
        AccessControlMaxAge, AccessControlRequestHeaders,
        AccessControlRequestMethod, Allow, Authorization, Basic, Bearer,
        CacheControl, Connection, ContentDisposition, ContentEncoding,
        ContentLanguage, ContentLength, ContentRange, ContentType, Date, ETag,
        EntityTag, Expires, From, Headers, Host, HttpDate, IfModifiedSince,
        IfUnmodifiedSince, LastModified, Location, Origin, Prefer,
        PreferenceApplied, Protocol, Quality, QualityItem, Referer,
        StrictTransportSecurity, TransferEncoding, Upgrade, UserAgent,
        AccessControlAllowOrigin, ByteRangeSpec, CacheDirective, Charset,
        ConnectionOption, ContentRangeSpec, DispositionParam, DispositionType,
        Encoding, Expect, IfMatch, IfNoneMatch, IfRange, Pragma, Preference,
        ProtocolName, Range, RangeUnit, ReferrerPolicy, Vary, Scheme, q, qitem
    }

    import_hyper_headers! {
        Accept, AccessControlAllowCredentials, AccessControlAllowHeaders,
        AccessControlAllowMethods, AccessControlAllowOrigin,
        AccessControlExposeHeaders, AccessControlMaxAge,
        AccessControlRequestHeaders, AccessControlRequestMethod, AcceptCharset,
        AcceptEncoding, AcceptLanguage, AcceptRanges, Allow, CacheControl,
        Connection, ContentDisposition, ContentEncoding, ContentLanguage,
        ContentLength, ContentRange, Date, ETag, Expect, Expires, Host, IfMatch,
        IfModifiedSince, IfNoneMatch, IfRange, IfUnmodifiedSince, LastModified,
        Location, Origin, Pragma, Prefer, PreferenceApplied, Range, Referer,
        ReferrerPolicy, StrictTransportSecurity, TransferEncoding, Upgrade,
        UserAgent, Vary
    }
}
