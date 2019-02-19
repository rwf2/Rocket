//! Re-exported hyper HTTP library types.
//!
//! All types that are re-exported from Hyper reside inside of this module.
//! These types will, with certainty, be removed with time, but they reside here
//! while necessary.

extern crate hyper;

#[doc(hidden)]
pub use self::hyper::server::Handler;
#[doc(hidden)]
pub use self::hyper::server::Request;
#[doc(hidden)]
pub use self::hyper::server::Response;
#[doc(hidden)]
pub use self::hyper::server::Server;

#[doc(hidden)]
pub use self::hyper::net;

#[doc(hidden)]
pub use self::hyper::buffer;
#[doc(hidden)]
pub use self::hyper::error::Error;
#[doc(hidden)]
pub use self::hyper::http::h1;
#[doc(hidden)]
pub use self::hyper::method::Method;
#[doc(hidden)]
pub use self::hyper::status::StatusCode;
#[doc(hidden)]
pub use self::hyper::uri::RequestUri;

pub use self::hyper::mime;

/// Type alias to `self::hyper::Response<'a, self::hyper::net::Fresh>`.
#[doc(hidden)]
pub type FreshResponse<'a> = self::Response<'a, self::net::Fresh>;

/// Reexported Hyper header types.
pub mod header {
    use Header;

    use super::hyper::header::Header as HyperHeaderTrait;

    macro_rules! import_hyper_items {
        ($($item:ident),*) => ($(pub use super::hyper::header::$item;)*)
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
