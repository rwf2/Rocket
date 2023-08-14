//! Contains types that set the status code and corresponding headers of a
//! response.
//!
//! # Responding
//!
//! Types in this module designed to make it easier to construct correct
//! responses with a given status code. Each type takes in the minimum number of
//! parameters required to construct a correct response. Some types take in
//! responders; when they do, the responder finalizes the response by writing
//! out additional headers and, importantly, the body of the response.
//!
//! The [`Custom`] type allows responding with _any_ `Status` but _does not_
//! ensure that all of the required headers are present. As a convenience,
//! `(Status, R)` where `R: Responder` is _also_ a `Responder`, identical to
//! `Custom`.
//!
//! ```rust
//! # extern crate rocket;
//! # use rocket::get;
//! use rocket::http::Status;
//!
//! #[get("/")]
//! fn index() -> (Status, &'static str) {
//!     (Status::NotFound, "Hey, there's no index!")
//! }
//! ```

use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use std::borrow::Cow;

use crate::request::Request;
use crate::response::{self, Responder, Response};
use crate::http::Status;

/// Sets the status of the response to 201 Created.
///
/// Sets the `Location` header and optionally the `ETag` header in the response.
/// The body of the response, which identifies the created resource, can be set
/// via the builder methods [`Created::body()`] and [`Created::tagged_body()`].
/// While both builder methods set the responder, the [`Created::tagged_body()`]
/// additionally computes a hash for the responder which is used as the value of
/// the `ETag` header when responding.
///
/// # Example
///
/// ```rust
/// use rocket::response::status;
///
/// let response = status::Created::new("http://myservice.com/resource.json")
///     .tagged_body("{ 'resource': 'Hello, world!' }");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Created<R>(Cow<'static, str>, Option<R>, Option<u64>);

impl<'r, R> Created<R> {
    /// Constructs a `Created` response with a `location` and no body.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::{get, routes, local::blocking::Client};
    /// use rocket::response::status;
    ///
    /// #[get("/")]
    /// fn create() -> status::Created<&'static str> {
    ///     status::Created::new("http://myservice.com/resource.json")
    /// }
    ///
    /// # let client = Client::debug_with(routes![create]).unwrap();
    /// let response = client.get("/").dispatch();
    ///
    /// let loc = response.headers().get_one("Location");
    /// assert_eq!(loc, Some("http://myservice.com/resource.json"));
    /// assert!(response.body().is_none());
    /// ```
    pub fn new<L: Into<Cow<'static, str>>>(location: L) -> Self {
        Created(location.into(), None, None)
    }

    /// Adds `responder` as the body of `self`.
    ///
    /// Unlike [`tagged_body()`](self::Created::tagged_body()), this method
    /// _does not_ result in an `ETag` header being set in the response.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::{get, routes, local::blocking::Client};
    /// use rocket::response::status;
    ///
    /// #[get("/")]
    /// fn create() -> status::Created<&'static str> {
    ///     status::Created::new("http://myservice.com/resource.json")
    ///         .body("{ 'resource': 'Hello, world!' }")
    /// }
    ///
    /// # let client = Client::debug_with(routes![create]).unwrap();
    /// let response = client.get("/").dispatch();
    ///
    /// let loc = response.headers().get_one("Location");
    /// assert_eq!(loc, Some("http://myservice.com/resource.json"));
    ///
    /// let etag = response.headers().get_one("ETag");
    /// assert_eq!(etag, None);
    ///
    /// let body = response.into_string();
    /// assert_eq!(body.unwrap(), "{ 'resource': 'Hello, world!' }");
    /// ```
    pub fn body(mut self, responder: R) -> Self {
        self.1 = Some(responder);
        self
    }

    /// Adds `responder` as the body of `self`. Computes a hash of the
    /// `responder` to be used as the value of the `ETag` header.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::{get, routes, local::blocking::Client};
    /// use rocket::response::status;
    ///
    /// #[get("/")]
    /// fn create() -> status::Created<&'static str> {
    ///     status::Created::new("http://myservice.com/resource.json")
    ///         .tagged_body("{ 'resource': 'Hello, world!' }")
    /// }
    ///
    /// # let client = Client::debug_with(routes![create]).unwrap();
    /// let response = client.get("/").dispatch();
    ///
    /// let loc = response.headers().get_one("Location");
    /// assert_eq!(loc, Some("http://myservice.com/resource.json"));
    ///
    /// let etag = response.headers().get_one("ETag");
    /// assert_eq!(etag, Some(r#""13046220615156895040""#));
    ///
    /// let body = response.into_string();
    /// assert_eq!(body.unwrap(), "{ 'resource': 'Hello, world!' }");
    /// ```
    pub fn tagged_body(mut self, responder: R) -> Self where R: Hash {
        let mut hasher = &mut DefaultHasher::default();
        responder.hash(&mut hasher);
        let hash = hasher.finish();
        self.1 = Some(responder);
        self.2 = Some(hash);
        self
    }
}

/// Sets the status code of the response to 201 Created. Sets the `Location`
/// header to the parameter in the [`Created::new()`] constructor.
///
/// The optional responder, set via [`Created::body()`] or
/// [`Created::tagged_body()`] finalizes the response if it exists. The wrapped
/// responder should write the body of the response so that it contains
/// information about the created resource. If no responder is provided, the
/// response body will be empty.
///
/// In addition to setting the status code, `Location` header, and finalizing
/// the response with the `Responder`, the `ETag` header is set conditionally if
/// a hashable `Responder` is provided via [`Created::tagged_body()`]. The `ETag`
/// header is set to a hash value of the responder.
impl<'r, 'o: 'r, R: Responder<'r, 'o>> Responder<'r, 'o> for Created<R> {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'o> {
        let mut response = Response::build();
        if let Some(responder) = self.1 {
            response.merge(responder.respond_to(req)?);
        }

        if let Some(hash) = self.2 {
            response.raw_header("ETag", format!(r#""{}""#, hash));
        }

        response.status(Status::Created)
            .raw_header("Location", self.0)
            .ok()
    }
}

/// Sets the status of the response to 204 No Content.
///
/// The response body will be empty.
///
/// # Example
///
/// A 204 No Content response:
///
/// ```rust
/// # use rocket::get;
/// use rocket::response::status;
///
/// #[get("/")]
/// fn foo() -> status::NoContent {
///     status::NoContent
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct NoContent;

/// Sets the status code of the response to 204 No Content.
impl<'r> Responder<'r, 'static> for NoContent {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        Response::build().status(Status::NoContent).ok()
    }
}

/// Creates a response with a status code and underlying responder.
///
/// Note that this is equivalent to `(Status, R)`.
///
/// # Example
///
/// ```rust
/// # use rocket::get;
/// use rocket::response::status;
/// use rocket::http::Status;
///
/// #[get("/")]
/// fn handler() -> status::Custom<&'static str> {
///     status::Custom(Status::ImATeapot, "Hi!")
/// }
///
/// // This is equivalent to the above.
/// #[get("/")]
/// fn handler2() -> (Status, &'static str) {
///     (Status::ImATeapot, "Hi!")
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Custom<R>(pub Status, pub R);

/// Sets the status code of the response and then delegates the remainder of the
/// response to the wrapped responder.
impl<'r, 'o: 'r, R: Responder<'r, 'o>> Responder<'r, 'o> for Custom<R> {
    #[inline]
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'o> {
        Response::build_from(self.1.respond_to(req)?)
            .status(self.0)
            .ok()
    }
}

impl<'r, 'o: 'r, R: Responder<'r, 'o>> Responder<'r, 'o> for (Status, R) {
    #[inline(always)]
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'o> {
        Custom(self.0, self.1).respond_to(request)
    }
}

macro_rules! status_response {
    ($T:ident $kind:expr) => {
        /// Sets the status of the response to
        #[doc = concat!($kind, concat!(" ([`Status::", stringify!($T), "`])."))]
        ///
        /// The remainder of the response is delegated to `self.0`.
        /// # Examples
        ///
        /// A
        #[doc = $kind]
        /// response without a body:
        ///
        /// ```rust
        /// # use rocket::get;
        /// use rocket::response::status;
        ///
        /// #[get("/")]
        #[doc = concat!("fn handler() -> status::", stringify!($T), "<()> {")]
        #[doc = concat!("    status::", stringify!($T), "(())")]
        /// }
        /// ```
        ///
        /// A
        #[doc = $kind]
        /// response _with_ a body:
        ///
        /// ```rust
        /// # use rocket::get;
        /// use rocket::response::status;
        ///
        /// #[get("/")]
        #[doc = concat!("fn handler() -> status::", stringify!($T), "<&'static str> {")]
        #[doc = concat!("    status::", stringify!($T), "(\"body\")")]
        /// }
        /// ```
        #[derive(Debug, Clone, PartialEq)]
        pub struct $T<R>(pub R);

        impl<'r, 'o: 'r, R: Responder<'r, 'o>> Responder<'r, 'o> for $T<R> {
            #[inline(always)]
            fn respond_to(self, req: &'r Request<'_>) -> response::Result<'o> {
                Custom(Status::$T, self.0).respond_to(req)
            }
        }
    }
}

status_response!(Accepted "202 Accepted");
status_response!(BadRequest "400 Bad Request");
status_response!(Unauthorized "401 Unauthorized");
status_response!(Forbidden "403 Forbidden");
status_response!(NotFound "404 NotFound");
status_response!(Conflict "409 Conflict");

// The following are unimplemented.
// 206 Partial Content (variant), 203 Non-Authoritative Information (headers).
