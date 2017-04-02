use std::convert::AsRef;

use time::Duration;

use outcome::IntoOutcome;
use response::{Response, Responder};
use request::{self, Request, FromRequest};
use http::{Status, Cookie};

// The name of the actual flash cookie.
const FLASH_COOKIE_NAME: &'static str = "_flash";

/// Sets a "flash" cookie that will be removed when it is accessed. The
/// anologous request type is
/// [FlashMessage](/rocket/request/type.FlashMessage.html).
///
/// This type makes it easy to send messages across requests. It is typically
/// used for "status" messages after redirects. For instance, if a user attempts
/// to visit a page he/she does not have access to, you may want to redirect the
/// user to a safe place and show a message indicating what happened on the
/// redirected page. The message should only persist for a single request. This
/// can be accomplished with this type.
///
/// # Usage
///
/// Each `Flash` message consists of a `name` and some `msg` contents. A generic
/// constructor ([new](#method.new)) can be used to construct a message with any
/// name, while the [warning](#method.warning), [success](#method.success), and
/// [error](#method.error) constructors create messages with the corresponding
/// names.
///
/// Messages can be retrieved on the request side via the
/// [FlashMessage](/rocket/request/type.FlashMessage.html) type and the
/// [name](#method.name) and [msg](#method.msg) methods.
///
/// # Response
///
/// The `Responder` implementation for `Flash` sets the message cookie and then
/// uses the passed in responder `res` to complete the response. In other words,
/// it simply sets a cookie and delagates the rest of the response handling to
/// the wrapped responder.
///
/// # Example
///
/// The following complete Rocket application illustrates the use of a `Flash`
/// message on both the request and response sides.
///
/// ```rust
/// # #![feature(plugin)]
/// # #![plugin(rocket_codegen)]
/// #
/// # extern crate rocket;
/// #
/// use rocket::response::{Flash, Redirect};
/// use rocket::request::FlashMessage;
/// use rocket::http::RawStr;
///
/// #[post("/login/<name>")]
/// fn login(name: &RawStr) -> Result<&'static str, Flash<Redirect>> {
///     if name == "special_user" {
///         Ok("Hello, special user!")
///     } else {
///         Err(Flash::error(Redirect::to("/"), "Invalid username."))
///     }
/// }
///
/// #[get("/")]
/// fn index(flash: Option<FlashMessage>) -> String {
///     flash.map(|msg| format!("{}: {}", msg.name(), msg.msg()))
///          .unwrap_or_else(|| "Welcome!".to_string())
/// }
///
/// fn main() {
/// # if false { // We don't actually want to launch the server in an example.
///     rocket::ignite().mount("/", routes![login, index]).launch();
/// # }
/// }
/// ```
///
/// On the response side (in `login`), a `Flash` error message is set if some
/// fictional authentication failed, and the user is redirected to `"/"`. On the
/// request side (in `index`), the handler emits the flash message if there is
/// one and otherwise emits a standard welcome message. Note that if the user
/// were to refresh the index page after viewing a flash message, the user would
/// receive the standard welcome message.
#[derive(Debug)]
pub struct Flash<R> {
    name: String,
    message: String,
    responder: R,
}

impl<'r, R: Responder<'r>> Flash<R> {
    /// Constructs a new `Flash` message with the given `name`, `msg`, and
    /// underlying `responder`.
    ///
    /// # Examples
    ///
    /// Construct a "suggestion" message with contents "Try this out!" that
    /// redirects to "/".
    ///
    /// ```rust
    /// use rocket::response::{Redirect, Flash};
    ///
    /// # #[allow(unused_variables)]
    /// let msg = Flash::new(Redirect::to("/"), "suggestion", "Try this out!");
    /// ```
    pub fn new<N: AsRef<str>, M: AsRef<str>>(res: R, name: N, msg: M) -> Flash<R> {
        Flash {
            name: name.as_ref().to_string(),
            message: msg.as_ref().to_string(),
            responder: res,
        }
    }

    /// Constructs a "success" `Flash` message with the given `responder` and
    /// `msg`.
    ///
    /// # Examples
    ///
    /// Construct a "success" message with contents "It worked!" that redirects
    /// to "/".
    ///
    /// ```rust
    /// use rocket::response::{Redirect, Flash};
    ///
    /// # #[allow(unused_variables)]
    /// let msg = Flash::success(Redirect::to("/"), "It worked!");
    /// ```
    pub fn success<S: AsRef<str>>(responder: R, msg: S) -> Flash<R> {
        Flash::new(responder, "success", msg)
    }

    /// Constructs a "warning" `Flash` message with the given `responder` and
    /// `msg`.
    ///
    /// # Examples
    ///
    /// Construct a "warning" message with contents "Watch out!" that redirects
    /// to "/".
    ///
    /// ```rust
    /// use rocket::response::{Redirect, Flash};
    ///
    /// # #[allow(unused_variables)]
    /// let msg = Flash::warning(Redirect::to("/"), "Watch out!");
    /// ```
    pub fn warning<S: AsRef<str>>(responder: R, msg: S) -> Flash<R> {
        Flash::new(responder, "warning", msg)
    }

    /// Constructs an "error" `Flash` message with the given `responder` and
    /// `msg`.
    ///
    /// # Examples
    ///
    /// Construct an "error" message with contents "Whoops!" that redirects
    /// to "/".
    ///
    /// ```rust
    /// use rocket::response::{Redirect, Flash};
    ///
    /// # #[allow(unused_variables)]
    /// let msg = Flash::error(Redirect::to("/"), "Whoops!");
    /// ```
    pub fn error<S: AsRef<str>>(responder: R, msg: S) -> Flash<R> {
        Flash::new(responder, "error", msg)
    }

    fn cookie(&self) -> Cookie<'static> {
        let content = format!("{}{}{}", self.name.len(), self.name, self.message);
        Cookie::build(FLASH_COOKIE_NAME, content)
            .max_age(Duration::minutes(5))
            .path("/")
            .finish()
    }
}

/// Sets the message cookie and then uses the wrapped responder to complete the
/// response. In other words, simply sets a cookie and delagates the rest of the
/// response handling to the wrapped responder. As a result, the `Outcome` of
/// the response is the `Outcome` of the wrapped `Responder`.
impl<'r, R: Responder<'r>> Responder<'r> for Flash<R> {
    fn respond(self) -> Result<Response<'r>, Status> {
        trace_!("Flash: setting message: {}:{}", self.name, self.message);
        let cookie = self.cookie();
        Response::build_from(self.responder.respond()?)
            .header_adjoin(&cookie)
            .ok()
    }
}

impl Flash<()> {
    /// Constructs a new message with the given name and message.
    fn named(name: &str, msg: &str) -> Flash<()> {
        Flash {
            name: name.to_string(),
            message: msg.to_string(),
            responder: (),
        }
    }

    /// Returns the `name` of this message.
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Returns the `msg` contents of this message.
    pub fn msg(&self) -> &str {
        self.message.as_str()
    }
}

/// Retrieves a flash message from a flash cookie and deletes the flash cookie.
/// If there is no flash cookie, an empty `Err` is returned.
///
/// The suggested use is through an `Option` and the `FlashMessage` type alias
/// in `request`: `Option<FlashMessage>`.
impl<'a, 'r> FromRequest<'a, 'r> for Flash<()> {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
        trace_!("Flash: attemping to retrieve message.");
        let r = request.cookies().get(FLASH_COOKIE_NAME).ok_or(()).and_then(|cookie| {
            trace_!("Flash: retrieving message: {:?}", cookie);

            // Parse the flash message.
            let content = cookie.value();
            let (len_str, rest) = match content.find(|c: char| !c.is_digit(10)) {
                Some(i) => (&content[..i], &content[i..]),
                None => (content, ""),
            };

            let name_len: usize = len_str.parse().map_err(|_| ())?;
            let (name, msg) = (&rest[..name_len], &rest[name_len..]);
            Ok(Flash::named(name, msg))
        });

        // If we found a flash cookie, delete it from the jar.
        if r.is_ok() {
            let cookie = Cookie::build(FLASH_COOKIE_NAME, "").path("/").finish();
            request.cookies().remove(cookie);
        }

        r.into_outcome()
    }
}
