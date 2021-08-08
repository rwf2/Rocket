//! Re-exported hyper HTTP library types.
//!
//! All types that are re-exported from Hyper reside inside of this module.
//! These types will, with certainty, be removed with time, but they reside here
//! while necessary.

#[doc(hidden)] pub use hyper::*;
#[doc(hidden)] pub use http::*;

/// Reexported http header types.
pub mod header {
    macro_rules! import_http_headers {
        ($($name:ident),*) => ($(
            pub use hyper::header::$name as $name;
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
}
