use std::fmt;
use std::io::Cursor;

use crate::response::Response;
use crate::request::Request;
use crate::http::{Status, ContentType, uri};
use crate::catcher::{Handler, BoxFuture};

use yansi::Paint;

/// An error catching route.
///
/// Catchers are routes that run when errors are produced by the application.
/// They consist of a [`Handler`] and an optional status code to match against
/// arising errors. Errors arise from the the following sources:
///
///   * A failing guard.
///   * A failing responder.
///   * Routing failure.
///
/// Each failure is paired with a status code. Guards and responders indicate
/// the status code themselves via their `Err` return value while a routing
/// failure is always a `404`. Rocket invokes the error handler for the catcher
/// with the error's status code.
///
/// ### Error Handler Restrictions
///
/// Because error handlers are a last resort, they should not fail to produce a
/// response. If an error handler _does_ fail, Rocket invokes its default `500`
/// error catcher. Error handlers cannot forward.
///
/// # Routing
///
/// An error arising from a particular request _matches_ a catcher _iff_:
///
///  * It is a default catcher _or_ has a status code matching the error code.
///  * Its base is a prefix of the normalized/decoded request URI path.
///
/// A _default_ catcher is a catcher with no explicit status code: `None`. The
/// catcher's _base_ is provided as the first argument to
/// [`Rocket::register()`](crate::Rocket::register()).
///
/// # Collisions
///
/// Two catchers are said to _collide_ if there exists an error that matches
/// both catchers. Colliding catchers present a routing ambiguity and are thus
/// disallowed by Rocket. Because catchers can be constructed dynamically,
/// collision checking is done at [`ignite`](crate::Rocket::ignite()) time,
/// after it becomes statically impossible to register any more catchers on an
/// instance of `Rocket`.
///
/// ### Built-In Default
///
/// Rocket's provides a built-in default catcher that can handle all errors. It
/// produces HTML or JSON, depending on the value of the `Accept` header. As
/// such, catchers only need to be registered if an error needs to be handled in
/// a custom fashion. The built-in default never conflicts with any
/// user-registered catchers.
///
/// # Code Generation
///
/// Catchers should rarely be constructed or used directly. Instead, they are
/// typically generated via the [`catch`] attribute, as follows:
///
/// ```rust,no_run
/// #[macro_use] extern crate rocket;
///
/// use rocket::Request;
/// use rocket::http::Status;
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
/// #[catch(default)]
/// fn default(status: Status, req: &Request) -> String {
///     format!("{} ({})", status, req.uri())
/// }
///
/// #[launch]
/// fn rocket() -> _ {
///     rocket::build().register("/", catchers![internal_error, not_found, default])
/// }
/// ```
///
/// A function decorated with `#[catch]` may take zero, one, or two arguments.
/// It's type signature must be one of the following, where `R:`[`Responder`]:
///
///   * `fn() -> R`
///   * `fn(`[`&Request`]`) -> R`
///   * `fn(`[`Status`]`, `[`&Request`]`) -> R`
///
/// See the [`catch`] documentation for full details.
///
/// [`catch`]: crate::catch
/// [`Responder`]: crate::response::Responder
/// [`&Request`]: crate::request::Request
/// [`Status`]: crate::http::Status
#[derive(Clone)]
pub struct Catcher {
    /// The name of this catcher, if one was given.
    pub name: Option<Cow<'static, str>>,

    /// The mount point.
    pub base: uri::Origin<'static>,

    /// The HTTP status to match against if this route is not `default`.
    pub code: Option<u16>,

    /// The catcher's associated error handler.
    pub handler: Box<dyn Handler>,
}

impl Catcher {
    /// Creates a catcher for the given `status`, or a default catcher if
    /// `status` is `None`, using the given error handler. This should only be
    /// used when routing manually.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rocket::request::Request;
    /// use rocket::catcher::{Catcher, BoxFuture};
    /// use rocket::response::Responder;
    /// use rocket::http::Status;
    ///
    /// fn handle_404<'r>(status: Status, req: &'r Request<'_>) -> BoxFuture<'r> {
    ///    let res = (status, format!("404: {}", req.uri()));
    ///    Box::pin(async move { res.respond_to(req) })
    /// }
    ///
    /// fn handle_500<'r>(_: Status, req: &'r Request<'_>) -> BoxFuture<'r> {
    ///     Box::pin(async move{ "Whoops, we messed up!".respond_to(req) })
    /// }
    ///
    /// fn handle_default<'r>(status: Status, req: &'r Request<'_>) -> BoxFuture<'r> {
    ///    let res = (status, format!("{}: {}", status, req.uri()));
    ///    Box::pin(async move { res.respond_to(req) })
    /// }
    ///
    /// let not_found_catcher = Catcher::new(404, handle_404);
    /// let internal_server_error_catcher = Catcher::new(500, handle_500);
    /// let default_error_catcher = Catcher::new(None, handle_default);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `code` is not in the HTTP status code error range `[400,
    /// 600)`.
    #[inline(always)]
    pub fn new<S, H>(code: S, handler: H) -> Catcher
        where S: Into<Option<u16>>, H: Handler
    {
        let code = code.into();
        if let Some(code) = code {
            assert!(code >= 400 && code < 600);
        }

        Catcher {
            name: None,
            base: uri::Origin::ROOT,
            handler: Box::new(handler),
            code,
        }
    }

    /// Maps the `base` of this catcher using `mapper`, returning a new
    /// `Catcher` with the returned base.
    ///
    /// `mapper` is called with the current base. The returned `String` is used
    /// as the new base if it is a valid URI. If the returned base URI contains
    /// a query, it is ignored. Returns an error if the base produced by
    /// `mapper` is not a valid origin URI.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::request::Request;
    /// use rocket::catcher::{Catcher, BoxFuture};
    /// use rocket::response::Responder;
    /// use rocket::http::Status;
    ///
    /// fn handle_404<'r>(status: Status, req: &'r Request<'_>) -> BoxFuture<'r> {
    ///    let res = (status, format!("404: {}", req.uri()));
    ///    Box::pin(async move { res.respond_to(req) })
    /// }
    ///
    /// let catcher = Catcher::new(404, handle_404);
    /// assert_eq!(catcher.base.path(), "/");
    ///
    /// let catcher = catcher.map_base(|_| format!("/bar")).unwrap();
    /// assert_eq!(catcher.base.path(), "/bar");
    ///
    /// let catcher = catcher.map_base(|base| format!("/foo{}", base)).unwrap();
    /// assert_eq!(catcher.base.path(), "/foo/bar");
    ///
    /// let catcher = catcher.map_base(|base| format!("/foo ? {}", base));
    /// assert!(catcher.is_err());
    /// ```
    pub fn map_base<'a, F>(
        mut self,
        mapper: F
    ) -> std::result::Result<Self, uri::Error<'static>>
        where F: FnOnce(uri::Origin<'a>) -> String
    {
        self.base = uri::Origin::parse_owned(mapper(self.base))?.into_normalized();
        self.base.clear_query();
        Ok(self)
    }
}

impl Default for Catcher {
    fn default() -> Self {
        fn handler<'r>(s: Status, req: &'r Request<'_>) -> BoxFuture<'r> {
            Box::pin(async move { Ok(default_handler(s, req)) })
        }

        let mut catcher = Catcher::new(None, handler);
        catcher.name = Some("<Rocket Catcher>".into());
        catcher
    }
}

/// Information generated by the `catch` attribute during codegen.
#[doc(hidden)]
pub struct StaticInfo {
    /// The catcher's name, i.e, the name of the function.
    pub name: &'static str,
    /// The catcher's status code.
    pub code: Option<u16>,
    /// The catcher's handler, i.e, the annotated function.
    pub handler: for<'r> fn(Status, &'r Request<'_>) -> BoxFuture<'r>,
}

#[doc(hidden)]
impl From<StaticInfo> for Catcher {
    #[inline]
    fn from(info: StaticInfo) -> Catcher {
        let mut catcher = Catcher::new(info.code, info.handler);
        catcher.name = Some(info.name.into());
        catcher
    }
}

impl fmt::Display for Catcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref n) = self.name {
            write!(f, "{}{}{} ", Paint::cyan("("), Paint::white(n), Paint::cyan(")"))?;
        }

        if self.base.path() != "/" {
            write!(f, "{} ", Paint::green(self.base.path()))?;
        }

        match self.code {
            Some(code) => write!(f, "{}", Paint::blue(code)),
            None => write!(f, "{}", Paint::blue("default"))
        }
    }
}

impl fmt::Debug for Catcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Catcher")
            .field("name", &self.name)
            .field("base", &self.base)
            .field("code", &self.code)
            .finish()
    }
}

macro_rules! html_error_template {
    ($code:expr, $reason:expr, $description:expr) => (
        concat!(
r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <title>"#, $code, " ", $reason, r#"</title>
</head>
<body align="center">
    <div role="main" align="center">
        <h1>"#, $code, ": ", $reason, r#"</h1>
        <p>"#, $description, r#"</p>
        <hr />
    </div>
    <div role="contentinfo" align="center">
        <small>Rocket</small>
    </div>
</body>
</html>"#
        )
    )
}

macro_rules! json_error_template {
    ($code:expr, $reason:expr, $description:expr) => (
        concat!(
r#"{
  "error": {
    "code": "#, $code, r#",
    "reason": ""#, $reason, r#"",
    "description": ""#, $description, r#""
  }
}"#
        )
    )
}

// This is unfortunate, but the `{`, `}` above make it unusable for `format!`.
macro_rules! json_error_fmt_template {
    ($code:expr, $reason:expr, $description:expr) => (
        concat!(
r#"{{
  "error": {{
    "code": "#, $code, r#",
    "reason": ""#, $reason, r#"",
    "description": ""#, $description, r#""
  }}
}}"#
        )
    )
}

macro_rules! default_handler_fn {
    ($($code:expr, $reason:expr, $description:expr),+) => (
        use std::borrow::Cow;

        pub(crate) fn default_handler<'r>(
            status: Status,
            req: &'r Request<'_>
        ) -> Response<'r> {
            let preferred = req.accept().map(|a| a.preferred());
            let (mime, text) = if preferred.map_or(false, |a| a.is_json()) {
                let json: Cow<'_, str> = match status.code {
                    $($code => json_error_template!($code, $reason, $description).into(),)*
                    code => format!(json_error_fmt_template!("{}", "Unknown Error",
                            "An unknown error has occurred."), code).into()
                };

                (ContentType::JSON, json)
            } else {
                let html: Cow<'_, str> = match status.code {
                    $($code => html_error_template!($code, $reason, $description).into(),)*
                    code => format!(html_error_template!("{}", "Unknown Error",
                            "An unknown error has occurred."), code, code).into(),
                };

                (ContentType::HTML, html)
            };

            let mut r = Response::build().status(status).header(mime).finalize();
            match text {
                Cow::Owned(v) => r.set_sized_body(v.len(), Cursor::new(v)),
                Cow::Borrowed(v) => r.set_sized_body(v.len(), Cursor::new(v)),
            };

            r
        }
    )
}

default_handler_fn! {
    400, "Bad Request", "The request could not be understood by the server due \
        to malformed syntax.",
    401, "Unauthorized", "The request requires user authentication.",
    402, "Payment Required", "The request could not be processed due to lack of payment.",
    403, "Forbidden", "The server refused to authorize the request.",
    404, "Not Found", "The requested resource could not be found.",
    405, "Method Not Allowed", "The request method is not supported for the requested resource.",
    406, "Not Acceptable", "The requested resource is capable of generating only content not \
        acceptable according to the Accept headers sent in the request.",
    407, "Proxy Authentication Required", "Authentication with the proxy is required.",
    408, "Request Timeout", "The server timed out waiting for the request.",
    409, "Conflict", "The request could not be processed because of a conflict in the request.",
    410, "Gone", "The resource requested is no longer available and will not be available again.",
    411, "Length Required", "The request did not specify the length of its content, which is \
        required by the requested resource.",
    412, "Precondition Failed", "The server does not meet one of the \
        preconditions specified in the request.",
    413, "Payload Too Large", "The request is larger than the server is \
        willing or able to process.",
    414, "URI Too Long", "The URI provided was too long for the server to process.",
    415, "Unsupported Media Type", "The request entity has a media type which \
        the server or resource does not support.",
    416, "Range Not Satisfiable", "The portion of the requested file cannot be \
        supplied by the server.",
    417, "Expectation Failed", "The server cannot meet the requirements of the \
        Expect request-header field.",
    418, "I'm a teapot", "I was requested to brew coffee, and I am a teapot.",
    421, "Misdirected Request", "The server cannot produce a response for this request.",
    422, "Unprocessable Entity", "The request was well-formed but was unable to \
        be followed due to semantic errors.",
    426, "Upgrade Required", "Switching to the protocol in the Upgrade header field is required.",
    428, "Precondition Required", "The server requires the request to be conditional.",
    429, "Too Many Requests", "Too many requests have been received recently.",
    431, "Request Header Fields Too Large", "The server is unwilling to process \
        the request because either an individual header field, or all the header \
        fields collectively, are too large.",
    451, "Unavailable For Legal Reasons", "The requested resource is unavailable \
        due to a legal demand to deny access to this resource.",
    500, "Internal Server Error", "The server encountered an internal error while \
        processing this request.",
    501, "Not Implemented", "The server either does not recognize the request \
        method, or it lacks the ability to fulfill the request.",
    503, "Service Unavailable", "The server is currently unavailable.",
    504, "Gateway Timeout", "The server did not receive a timely response from an upstream server.",
    510, "Not Extended", "Further extensions to the request are required for \
        the server to fulfill it."
}
