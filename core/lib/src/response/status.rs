//! Contains types that set the status code and corresponding headers of a
//! response.
//!
//! These types are designed to make it easier to respond correctly with a given
//! status code. Each type takes in the minimum number of parameters required to
//! construct a proper response with that status code. Some types take in
//! responders; when they do, the responder finalizes the response by writing
//! out additional headers and, importantly, the body of the response.

use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

use crate::request::Request;
use crate::response::{Responder, Response};
use crate::http::hyper::header;
use crate::http::Status;

/// Sets the status of the response to 201 (Created).
///
/// The `String` field is set as the value of the `Location` header in the
/// response. The optional `Responder` field is used to finalize the response.
///
/// # Example
///
/// ```rust
/// use rocket::response::status;
///
/// let url = "http://myservice.com/resource.json".to_string();
/// let content = "{ 'resource': 'Hello, world!' }";
/// # #[allow(unused_variables)]
/// let response = status::Created(url, Some(content));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Created<R>(pub String, pub Option<R>);

/// Sets the status code of the response to 201 Created. Sets the `Location`
/// header to the `String` parameter in the constructor.
///
/// The optional responder finalizes the response if it exists. The wrapped
/// responder should write the body of the response so that it contains
/// information about the created resource. If no responder is provided, the
/// response body will be empty.
impl<'r, R: Responder<'r>> Responder<'r> for Created<R> {
    default fn respond_to(self, req: &Request<'_>) -> Result<Response<'r>, Status> {
        let mut build = Response::build();
        if let Some(responder) = self.1 {
            build.merge(responder.respond_to(req)?);
        }

        build.status(Status::Created).header(header::Location(self.0)).ok()
    }
}

/// In addition to setting the status code, `Location` header, and finalizing
/// the response with the `Responder`, the `ETag` header is set conditionally if
/// a `Responder` is provided that implements `Hash`. The `ETag` header is set
/// to a hash value of the responder.
impl<'r, R: Responder<'r> + Hash> Responder<'r> for Created<R> {
    fn respond_to(self, req: &Request<'_>) -> Result<Response<'r>, Status> {
        let mut hasher = DefaultHasher::default();
        let mut build = Response::build();
        if let Some(responder) = self.1 {
            responder.hash(&mut hasher);
            let hash = hasher.finish().to_string();

            build.merge(responder.respond_to(req)?);
            build.header(header::ETag(header::EntityTag::strong(hash)));
        }

        build.status(Status::Created).header(header::Location(self.0)).ok()
    }
}

/// Sets the status of the response to 202 (Accepted).
///
/// If a responder is supplied, the remainder of the response is delegated to
/// it. If there is no responder, the body of the response will be empty.
///
/// # Examples
///
/// A 202 Accepted response without a body:
///
/// ```rust
/// use rocket::response::status;
///
/// # #[allow(unused_variables)]
/// let response = status::Accepted::<()>(None);
/// ```
///
/// A 202 Accepted response _with_ a body:
///
/// ```rust
/// use rocket::response::status;
///
/// # #[allow(unused_variables)]
/// let response = status::Accepted(Some("processing"));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Accepted<R>(pub Option<R>);

/// Sets the status code of the response to 202 Accepted. If the responder is
/// `Some`, it is used to finalize the response.
impl<'r, R: Responder<'r>> Responder<'r> for Accepted<R> {
    fn respond_to(self, req: &Request<'_>) -> Result<Response<'r>, Status> {
        let mut build = Response::build();
        if let Some(responder) = self.0 {
            build.merge(responder.respond_to(req)?);
        }

        build.status(Status::Accepted).ok()
    }
}

/// Sets the status of the response to 400 (Bad Request).
///
/// If a responder is supplied, the remainder of the response is delegated to
/// it. If there is no responder, the body of the response will be empty.
///
/// # Examples
///
/// A 400 Bad Request response without a body:
///
/// ```rust
/// use rocket::response::status;
///
/// # #[allow(unused_variables)]
/// let response = status::BadRequest::<()>(None);
/// ```
///
/// A 400 Bad Request response _with_ a body:
///
/// ```rust
/// use rocket::response::status;
///
/// # #[allow(unused_variables)]
/// let response = status::BadRequest(Some("error message"));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct BadRequest<R>(pub Option<R>);

/// Sets the status code of the response to 400 Bad Request. If the responder is
/// `Some`, it is used to finalize the response.
impl<'r, R: Responder<'r>> Responder<'r> for BadRequest<R> {
    fn respond_to(self, req: &Request<'_>) -> Result<Response<'r>, Status> {
        let mut build = Response::build();
        if let Some(responder) = self.0 {
            build.merge(responder.respond_to(req)?);
        }

        build.status(Status::BadRequest).ok()
    }
}

/// Sets the status of the response to 404 (Not Found).
///
/// The remainder of the response is delegated to the wrapped `Responder`.
///
/// # Example
///
/// ```rust
/// use rocket::response::status;
///
/// # #[allow(unused_variables)]
/// let response = status::NotFound("Sorry, I couldn't find it!");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct NotFound<R>(pub R);

/// Sets the status code of the response to 404 Not Found.
impl<'r, R: Responder<'r>> Responder<'r> for NotFound<R> {
    fn respond_to(self, req: &Request<'_>) -> Result<Response<'r>, Status> {
        Response::build_from(self.0.respond_to(req)?)
            .status(Status::NotFound)
            .ok()
    }
}

/// Creates a response with the given status code and underlying responder.
///
/// # Example
///
/// ```rust
/// use rocket::response::status;
/// use rocket::http::Status;
///
/// # #[allow(unused_variables)]
/// let response = status::Custom(Status::ImATeapot, "Hi!");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Custom<R>(pub Status, pub R);

/// Sets the status code of the response and then delegates the remainder of the
/// response to the wrapped responder.
impl<'r, R: Responder<'r>> Responder<'r> for Custom<R> {
    fn respond_to(self, req: &Request<'_>) -> Result<Response<'r>, Status> {
        Response::build_from(self.1.respond_to(req)?)
            .status(self.0)
            .ok()
    }
}

// The following are unimplemented.
// 206 Partial Content (variant), 203 Non-Authoritative Information (headers).
