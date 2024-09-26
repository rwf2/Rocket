use crate::{outcome::IntoOutcome, Request};
use super::FromRequest;

use headers::{Header as HHeader, HeaderValue as HHeaderValue};
use rocket_http::Status;

macro_rules! typed_headers_from_request {
    ($($name:ident),*) => ($(
        pub use crate::http::$name;

        #[rocket::async_trait]
        impl<'r> FromRequest<'r> for $name {
            type Error = headers::Error;
            async fn from_request(req: &'r Request<'_>) ->
                    crate::request::Outcome<Self, Self::Error> {
                req.headers().get($name::name().as_str()).next().or_forward(Status::NotFound)
                    .and_then(|h| HHeaderValue::from_str(h).or_error(Status::BadRequest))
                    .map_error(|(s, _)| (s, headers::Error::invalid()))
                    .and_then(|h| $name::decode(&mut std::iter::once(&h))
                        .or_forward(Status::BadRequest))
            }
        }
    )*)
}

macro_rules! generic_typed_headers_from_request {
($($name:ident<$bound:ident>),*) => ($(
    pub use crate::http::$name;

    #[rocket::async_trait]
    impl<'r, T1: 'static + $bound> FromRequest<'r> for $name<T1> {
        type Error = headers::Error;
        async fn from_request(req: &'r Request<'_>) -> crate::request::Outcome<Self, Self::Error> {
            req.headers().get($name::<T1>::name().as_str()).next()
                .or_forward(Status::NotFound)
                .and_then(|h| HHeaderValue::from_str(h).or_error(Status::BadRequest))
                .map_error(|(s, _)| (s, headers::Error::invalid()))
                .and_then(|h| $name::decode(&mut std::iter::once(&h))
                    .or_forward(Status::BadRequest))
        }
    }
)*)
}

// The following headers from 'headers' 0.4 are not imported, since they are
// provided by other Rocket features.

// * ContentType, // Content-Type header, defined in RFC7231
// * Cookie, // Cookie header, defined in RFC6265
// * Host, // The Host header.
// * Location, // Location header, defined in RFC7231
// * SetCookie, // Set-Cookie header, defined RFC6265

typed_headers_from_request! {
  AcceptRanges, // Accept-Ranges header, defined in RFC7233
  AccessControlAllowCredentials, // Access-Control-Allow-Credentials header, part of CORS
  AccessControlAllowHeaders, // Access-Control-Allow-Headers header, part of CORS
  AccessControlAllowMethods, // Access-Control-Allow-Methods header, part of CORS
  AccessControlAllowOrigin, // The Access-Control-Allow-Origin response header, part of CORS
  AccessControlExposeHeaders, // Access-Control-Expose-Headers header, part of CORS
  AccessControlMaxAge, // Access-Control-Max-Age header, part of CORS
  AccessControlRequestHeaders, // Access-Control-Request-Headers header, part of CORS
  AccessControlRequestMethod, // Access-Control-Request-Method header, part of CORS
  Age, // Age header, defined in RFC7234
  Allow, // Allow header, defined in RFC7231
  CacheControl, // Cache-Control header, defined in RFC7234 with extensions in RFC8246
  Connection, // Connection header, defined in RFC7230
  ContentDisposition, // A Content-Disposition header, (re)defined in RFC6266.
  ContentEncoding, // Content-Encoding header, defined in RFC7231
  ContentLength, // Content-Length header, defined in RFC7230
  ContentLocation, // Content-Location header, defined in RFC7231
  ContentRange, // Content-Range, described in RFC7233
  Date, // Date header, defined in RFC7231
  ETag, // ETag header, defined in RFC7232
  Expect, // The Expect header.
  Expires, // Expires header, defined in RFC7234
  IfMatch, // If-Match header, defined in RFC7232
  IfModifiedSince, // If-Modified-Since header, defined in RFC7232
  IfNoneMatch, // If-None-Match header, defined in RFC7232
  IfRange, // If-Range header, defined in RFC7233
  IfUnmodifiedSince, // If-Unmodified-Since header, defined in RFC7232
  LastModified, // Last-Modified header, defined in RFC7232
  Origin, // The Origin header.
  Pragma, // The Pragma header defined by HTTP/1.0.
  Range, // Range header, defined in RFC7233
  Referer, // Referer header, defined in RFC7231
  ReferrerPolicy, // Referrer-Policy header, part of Referrer Policy
  RetryAfter, // The Retry-After header.
  SecWebsocketAccept, // The Sec-Websocket-Accept header.
  SecWebsocketKey, // The Sec-Websocket-Key header.
  SecWebsocketVersion, // The Sec-Websocket-Version header.
  Server, // Server header, defined in RFC7231
  StrictTransportSecurity, // StrictTransportSecurity header, defined in RFC6797
  Te, // TE header, defined in RFC7230
  TransferEncoding, // Transfer-Encoding header, defined in RFC7230
  Upgrade, // Upgrade header, defined in RFC7230
  UserAgent, // User-Agent header, defined in RFC7231
  Vary // Vary header, defined in RFC7231
}

pub use headers::authorization::Credentials;

generic_typed_headers_from_request! {
    Authorization<Credentials>, // Authorization header, defined in RFC7235
    ProxyAuthorization<Credentials> // Proxy-Authorization header, defined in RFC7235
}
