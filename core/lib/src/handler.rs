//! Types and traits for request and error handlers and their return values.

use crate::data::Data;
use crate::request::Request;
use crate::response::{self, Response, Responder};
use crate::http::Status;
use crate::outcome;

/// Type alias for the `Outcome` of a `Handler`.
pub type Outcome<'r> = outcome::Outcome<Response<'r>, Status, Data>;

/// Trait implemented by types that can handle requests.
///
/// In general, you will never need to implement `Handler` manually or be
/// concerned about the `Handler` trait; Rocket's code generation handles
/// everything for you. You only need to learn about this trait if you want to
/// provide an external, library-based mechanism to handle requests where
/// request handling depends on input from the user. In other words, if you want
/// to write a plugin for Rocket that looks mostly like a static route but need
/// user provided state to make a request handling decision, you should consider
/// implementing a custom `Handler`.
///
/// # Example
///
/// Say you'd like to write a handler that changes its functionality based on an
/// enum value that the user provides:
///
/// ```rust
/// #[derive(Copy, Clone)]
/// enum Kind {
///     Simple,
///     Intermediate,
///     Complex,
/// }
/// ```
///
/// Such a handler might be written and used as follows:
///
/// ```rust
/// # #[derive(Copy, Clone)] enum Kind { Simple, Intermediate, Complex, }
/// use rocket::{Request, Data, Route, http::Method};
/// use rocket::handler::{self, Handler, Outcome};
///
/// #[derive(Clone)]
/// struct CustomHandler(Kind);
///
/// impl Handler for CustomHandler {
///     fn handle<'r>(&self, req: &'r Request, data: Data) -> Outcome<'r> {
///         match self.0 {
///             Kind::Simple => Outcome::from(req, "simple"),
///             Kind::Intermediate => Outcome::from(req, "intermediate"),
///             Kind::Complex => Outcome::from(req, "complex"),
///         }
///     }
/// }
///
/// impl Into<Vec<Route>> for CustomHandler {
///     fn into(self) -> Vec<Route> {
///         vec![Route::new(Method::Get, "/", self)]
///     }
/// }
///
/// fn main() {
/// # if false {
///     rocket::ignite()
///         .mount("/", CustomHandler(Kind::Simple))
///         .launch();
/// # }
/// }
/// ```
///
/// Note the following:
///
///   1. `CustomHandler` implements `Clone`. This is required so that
///      `CustomHandler` implements `Cloneable` automatically. The `Cloneable`
///      trait serves no other purpose but to ensure that every `Handler` can be
///      cloned, allowing `Route`s to be cloned.
///   2. `CustomHandler` implements `Into<Vec<Route>>`, allowing an instance to
///      be used directly as the second parameter to `rocket.mount()`.
///   3. Unlike static-function-based handlers, this custom handler can make use
///      of any internal state.
///
/// # Alternatives
///
/// The previous example could have been implemented using a combination of
/// managed state and a static route, as follows:
///
/// ```rust
/// # #![feature(proc_macro_hygiene, decl_macro)]
/// # #[macro_use] extern crate rocket;
/// #
/// # #[derive(Copy, Clone)]
/// # enum Kind {
/// #     Simple,
/// #     Intermediate,
/// #     Complex,
/// # }
/// #
/// use rocket::State;
///
/// #[get("/")]
/// fn custom_handler(state: State<Kind>) -> &'static str {
///     match *state {
///         Kind::Simple => "simple",
///         Kind::Intermediate => "intermediate",
///         Kind::Complex => "complex",
///     }
/// }
///
/// fn main() {
/// # if false {
///     rocket::ignite()
///         .mount("/", routes![custom_handler])
///         .manage(Kind::Simple)
///         .launch();
/// # }
/// }
/// ```
///
/// Pros:
///
///   * The handler is easier to implement since Rocket's code generation
///     ensures type-safety at all levels.
///
/// Cons:
///
///   * Only one `Kind` can be stored in managed state. As such, only one
///     variant of the custom handler can be used.
///   * The user must remember to manually call `rocket.manage(state)`.
///
/// Use this alternative when a single configuration is desired and your custom
/// handler is private to your application. For all other cases, a custom
/// `Handler` implementation is preferred.
pub trait Handler: Cloneable + Send + Sync + 'static {
    /// Called by Rocket when a `Request` with its associated `Data` should be
    /// handled by this handler.
    ///
    /// The variant of `Outcome` returned determines what Rocket does next. If
    /// the return value is a `Success(Response)`, the wrapped `Response` is
    /// used to respond to the client. If the return value is a
    /// `Failure(Status)`, the error catcher for `Status` is invoked to generate
    /// a response. Otherwise, if the return value is `Forward(Data)`, the next
    /// matching route is attempted. If there are no other matching routes, the
    /// `404` error catcher is invoked.
    fn handle<'r>(&self, request: &'r Request<'_>, data: Data) -> Outcome<'r>;
}

/// Unfortunate but necessary hack to be able to clone a `Box<Handler>`.
///
/// This trait should _never_ (and cannot, due to coherence) be implemented by
/// any type. Instead, implement `Clone`. All types that implement `Clone` and
/// `Handler` automatically implement `Cloneable`.
pub trait Cloneable {
    /// Clones `self`.
    fn clone_handler(&self) -> Box<dyn Handler>;
}

impl<T: Handler + Clone> Cloneable for T {
    #[inline(always)]
    fn clone_handler(&self) -> Box<dyn Handler> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn Handler> {
    #[inline(always)]
    fn clone(&self) -> Box<dyn Handler> {
        self.clone_handler()
    }
}

impl<F: Clone + Sync + Send + 'static> Handler for F
    where for<'r> F: Fn(&'r Request<'_>, Data) -> Outcome<'r>
{
    #[inline(always)]
    fn handle<'r>(&self, req: &'r Request<'_>, data: Data) -> Outcome<'r> {
        self(req, data)
    }
}

/// The type of an error handler.
pub type ErrorHandler = for<'r> fn(&'r Request<'_>) -> response::Result<'r>;

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
    pub fn from<T: Responder<'r>>(req: &Request<'_>, responder: T) -> Outcome<'r> {
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
