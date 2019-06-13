use crate::response;
use crate::handler::ErrorHandler;
use crate::codegen::StaticCatchInfo;
use crate::request::Request;

use std::fmt;
use yansi::Color::*;

/// An error catching route.
///
/// Catchers are routes that run when errors occur. They correspond directly
/// with the HTTP error status code they will be handling and are registered
/// with Rocket via [`Rocket::register()`](crate::Rocket::register()). For example,
/// to handle "404 not found" errors, a catcher for the "404" status code is
/// registered.
///
/// Because error handlers are only called when all routes are exhausted, they
/// should not fail nor forward. If an error catcher fails, the user will
/// receive no response. If an error catcher forwards, Rocket will respond with
/// an internal server error.
///
/// # Built-In Catchers
///
/// Rocket has many built-in, pre-registered default catchers. In particular,
/// Rocket has catchers for all of the following status codes: 400, 401, 402,
/// 403, 404, 405, 406, 407, 408, 409, 410, 411, 412, 413, 414, 415, 416, 417,
/// 418, 421, 426, 428, 429, 431, 451, 500, 501, 503, and 510. As such, catchers
/// only need to be registered if an error needs to be handled in a custom
/// fashion.
///
/// # Code Generation
///
/// Catchers should rarely be used directly. Instead, they are typically
/// declared using the `catch` decorator, as follows:
///
/// ```rust
/// #![feature(proc_macro_hygiene, decl_macro)]
///
/// #[macro_use] extern crate rocket;
///
/// use rocket::Request;
///
/// #[catch(500)]
/// fn internal_error() -> &'static str {
///     "Whoops! Looks like we messed up."
/// }
///
/// #[catch(404)]
/// fn not_found(req: &Request) -> String {
///     format!("I couldn't find '{}'. Try something else?", req.uri())
/// }
///
/// fn main() {
/// # if false { // We don't actually want to launch the server in an example.
///     rocket::ignite().register(catchers![internal_error, not_found]).launch();
/// # }
/// }
/// ```
///
/// A function decorated with `catch` must take exactly zero or one arguments.
/// If the catcher takes an argument, it must be of type [`&Request`](Request).
pub struct Catcher {
    /// The HTTP status code to match against.
    pub code: u16,
    /// The catcher's associated handler.
    pub handler: ErrorHandler,
    crate is_default: bool,
}

impl Catcher {
    /// Creates a catcher for the given status code using the given error
    /// handler. This should only be used when routing manually.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![allow(unused_variables)]
    /// use rocket::{Catcher, Request};
    /// use rocket::response::{Result, Responder};
    /// use rocket::response::status::Custom;
    /// use rocket::http::Status;
    ///
    /// fn handle_404<'r>(req: &'r Request) -> Result<'r> {
    ///     let res = Custom(Status::NotFound, format!("404: {}", req.uri()));
    ///     res.respond_to(req)
    /// }
    ///
    /// fn handle_500<'r>(req: &'r Request) -> Result<'r> {
    ///     "Whoops, we messed up!".respond_to(req)
    /// }
    ///
    /// let not_found_catcher = Catcher::new(404, handle_404);
    /// let internal_server_error_catcher = Catcher::new(500, handle_500);
    /// ```
    #[inline(always)]
    pub fn new(code: u16, handler: ErrorHandler) -> Catcher {
        Catcher { code, handler, is_default: false }
    }

    #[inline(always)]
    crate fn handle<'r>(&self, req: &'r Request<'_>) -> response::Result<'r> {
        (self.handler)(req)
    }

    #[inline(always)]
    fn new_default(code: u16, handler: ErrorHandler) -> Catcher {
        Catcher { code, handler, is_default: true, }
    }
}

#[doc(hidden)]
impl<'a> From<&'a StaticCatchInfo> for Catcher {
    fn from(info: &'a StaticCatchInfo) -> Catcher {
        Catcher::new(info.code, info.handler)
    }
}

impl fmt::Display for Catcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Blue.paint(&self.code))
    }
}

macro_rules! error_page_template {
    ($code:expr, $name:expr, $description:expr) => (
        concat!(r#"
            <!DOCTYPE html>
            <html>
            <head>
                <meta charset="utf-8">
                <title>"#, $code, " ", $name, r#"</title>
            </head>
            <body align="center">
                <div align="center">
                    <h1>"#, $code, ": ", $name, r#"</h1>
                    <p>"#, $description, r#"</p>
                    <hr />
                    <small>Rocket</small>
                </div>
            </body>
            </html>
        "#
        )
    )
}

macro_rules! default_catchers {
    ($($code:expr, $name:expr, $description:expr, $fn_name:ident),+) => (
        let mut map = HashMap::new();

        $(
            fn $fn_name<'r>(req: &'r Request<'_>) -> response::Result<'r> {
                status::Custom(Status::from_code($code).unwrap(),
                    content::Html(error_page_template!($code, $name, $description))
                ).respond_to(req)
            }

            map.insert($code, Catcher::new_default($code, $fn_name));
        )+

        map
    )
}

pub mod defaults {
    use super::Catcher;

    use std::collections::HashMap;

    use crate::request::Request;
    use crate::response::{self, content, status, Responder};
    use crate::http::Status;

    pub fn get() -> HashMap<u16, Catcher> {
        default_catchers! {
            400, "Bad Request", "The request could not be understood by the server due
                to malformed syntax.", handle_400,
            401, "Unauthorized", "The request requires user authentication.",
                handle_401,
            402, "Payment Required", "The request could not be processed due to lack of
                payment.", handle_402,
            403, "Forbidden", "The server refused to authorize the request.", handle_403,
            404, "Not Found", "The requested resource could not be found.", handle_404,
            405, "Method Not Allowed", "The request method is not supported for the
                requested resource.", handle_405,
            406, "Not Acceptable", "The requested resource is capable of generating
                only content not acceptable according to the Accept headers sent in the
                request.", handle_406,
            407, "Proxy Authentication Required", "Authentication with the proxy is
                required.", handle_407,
            408, "Request Timeout", "The server timed out waiting for the
                request.", handle_408,
            409, "Conflict", "The request could not be processed because of a conflict
                in the request.", handle_409,
            410, "Gone", "The resource requested is no longer available and will not be
                available again.", handle_410,
            411, "Length Required", "The request did not specify the length of its
                content, which is required by the requested resource.", handle_411,
            412, "Precondition Failed", "The server does not meet one of the
                preconditions specified in the request.", handle_412,
            413, "Payload Too Large", "The request is larger than the server is
                willing or able to process.", handle_413,
            414, "URI Too Long", "The URI provided was too long for the server to
                process.", handle_414,
            415, "Unsupported Media Type", "The request entity has a media type which
                the server or resource does not support.", handle_415,
            416, "Range Not Satisfiable", "The portion of the requested file cannot be
                supplied by the server.", handle_416,
            417, "Expectation Failed", "The server cannot meet the requirements of the
                Expect request-header field.", handle_417,
            418, "I'm a teapot", "I was requested to brew coffee, and I am a
                teapot.", handle_418,
            421, "Misdirected Request", "The server cannot produce a response for this
                request.", handle_421,
            422, "Unprocessable Entity", "The request was well-formed but was unable to
                be followed due to semantic errors.", handle_422,
            426, "Upgrade Required", "Switching to the protocol in the Upgrade header
                field is required.", handle_426,
            428, "Precondition Required", "The server requires the request to be
               conditional.", handle_428,
            429, "Too Many Requests", "Too many requests have been received
                recently.", handle_429,
            431, "Request Header Fields Too Large", "The server is unwilling to process
                the request because either an individual header field, or all
                the header fields collectively, are too large.", handle_431,
            451, "Unavailable For Legal Reasons", "The requested resource is
                unavailable due to a legal demand to deny access to this
                resource.", handle_451,
            500, "Internal Server Error", "The server encountered an internal error
                while processing this request.", handle_500,
            501, "Not Implemented", "The server either does not recognize the request
                method, or it lacks the ability to fulfill the request.", handle_501,
            503, "Service Unavailable", "The server is currently unavailable.",
                handle_503,
            504, "Gateway Timeout", "The server did not receive a timely
                response from an upstream server.", handle_504,
            510, "Not Extended", "Further extensions to the request are required for
                the server to fulfill it.", handle_510
        }
    }
}

