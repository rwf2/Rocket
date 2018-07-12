//! The types of request and error handlers and their return values.

use data::Data;
use request::Request;
use response::{self, Response, Responder};
use error::Error;
use http::Status;
use outcome;

/// Type alias for the `Outcome` of a `Handler`.
pub type Outcome<'r> = outcome::Outcome<Response<'r>, Status, Data>;

/// The type of a request handler.
pub type Handler = for<'r> fn(&'r Request, Data) -> Outcome<'r>;

/// The type of an error handler.
pub type ErrorHandler = for<'r> fn(Error, &'r Request) -> response::Result<'r>;

impl<'r> Outcome<'r> {
    /// Return the `Outcome` of response to `req` from `responder`.
    ///
    /// If the responder returns `Ok`, an outcome of `Success` is
    /// returned with the response. If the responder returns `Err`, an
    /// outcome of `Failure` is returned with the status code.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::{Request, Data};
    /// use rocket::handler::Outcome;
    ///
    /// fn str_responder(req: &Request, _: Data) -> Outcome<'static> {
    ///     Outcome::from(req, "Hello, world!")
    /// }
    /// ```
    #[inline]
    pub fn from<T: Responder<'r>>(req: &Request, responder: T) -> Outcome<'r> {
        match responder.respond_to(req) {
            Ok(response) => outcome::Outcome::Success(response),
            Err(status) => outcome::Outcome::Failure(status)
        }
    }

    /// Return an `Outcome` of `Failure` with the status code `code`. This is
    /// equivalent to `Outcome::Failure(code)`.
    ///
    /// This method exists to be used during manual routing where
    /// `rocket::handler::Outcome` is imported instead of `rocket::Outcome`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::{Request, Data};
    /// use rocket::handler::Outcome;
    /// use rocket::http::Status;
    ///
    /// fn bad_req_route(_: &Request, _: Data) -> Outcome<'static> {
    ///     Outcome::failure(Status::BadRequest)
    /// }
    /// ```
    #[inline(always)]
    pub fn failure(code: Status) -> Outcome<'static> {
        outcome::Outcome::Failure(code)
    }

    /// Return an `Outcome` of `Forward` with the data `data`. This is
    /// equivalent to `Outcome::Forward(data)`.
    ///
    /// This method exists to be used during manual routing where
    /// `rocket::handler::Outcome` is imported instead of `rocket::Outcome`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::{Request, Data};
    /// use rocket::handler::Outcome;
    ///
    /// fn always_forward(_: &Request, data: Data) -> Outcome<'static> {
    ///     Outcome::forward(data)
    /// }
    /// ```
    #[inline(always)]
    pub fn forward(data: Data) -> Outcome<'static> {
        outcome::Outcome::Forward(data)
    }
}
