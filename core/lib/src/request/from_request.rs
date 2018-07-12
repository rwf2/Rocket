use std::fmt::Debug;
use std::net::SocketAddr;

use router::Route;
use request::Request;
use outcome::{self, IntoOutcome};
use outcome::Outcome::*;

use http::{Status, ContentType, Accept, Method, Cookies};
use http::uri::Uri;

/// Type alias for the `Outcome` of a `FromRequest` conversion.
pub type Outcome<S, E> = outcome::Outcome<S, (Status, E), ()>;

impl<S, E> IntoOutcome<S, (Status, E), ()> for Result<S, E> {
    type Failure = Status;
    type Forward = ();

    #[inline]
    fn into_outcome(self, status: Status) -> Outcome<S, E> {
        match self {
            Ok(val) => Success(val),
            Err(err) => Failure((status, err))
        }
    }

    #[inline]
    fn or_forward(self, _: ()) -> Outcome<S, E> {
        match self {
            Ok(val) => Success(val),
            Err(_) => Forward(())
        }
    }
}

/// Trait implemented by request guards to derive a value from incoming
/// requests.
///
/// # Request Guards
///
/// A request guard is a type that represents an arbitrary validation policy.
/// The validation policy is implemented through `FromRequest`. In other words,
/// every type that implements `FromRequest` is a request guard.
///
/// Request guards appear as inputs to handlers. An arbitrary number of request
/// guards can appear as arguments in a route handler. Rocket will automatically
/// invoke the `FromRequest` implementation for request guards before calling
/// the handler. Rocket only dispatches requests to a handler when all of its
/// guards pass.
///
/// ## Example
///
/// The following dummy handler makes use of three request guards, `A`, `B`, and
/// `C`. An input type can be identified as a request guard if it is not named
/// in the route attribute. This is why, for instance, `param` is not a request
/// guard.
///
/// ```rust,ignore
/// #[get("/<param>")]
/// fn index(param: isize, a: A, b: B, c: C) -> ... { ... }
/// ```
///
/// Request guards always fire in left-to-right declaration order. In the
/// example above, for instance, the order will be `a` followed by `b` followed
/// by `c`. Failure is short-circuiting; if one guard fails, the remaining are
/// not attempted.
///
/// # Outcomes
///
/// The returned [Outcome](/rocket/outcome/index.html) of a `from_request` call
/// determines how the incoming request will be processed.
///
/// * **Success**(S)
///
///   If the `Outcome` is `Success`, then the `Success` value will be used as
///   the value for the corresponding parameter.  As long as all other guards
///   succeed, the request will be handled.
///
/// * **Failure**(Status, E)
///
///   If the `Outcome` is `Failure`, the request will fail with the given status
///   code and error. The designated error
///   [Catcher](/rocket/struct.Catcher.html) will be used to respond to the
///   request. Note that users can request types of `Result<S, E>` and
///   `Option<S>` to catch `Failure`s and retrieve the error value.
///
/// * **Forward**
///
///   If the `Outcome` is `Forward`, the request will be forwarded to the next
///   matching request. Note that users can request an `Option<S>` to catch
///   `Forward`s.
///
/// # Provided Implementations
///
/// Rocket implements `FromRequest` for several built-in types. Their behavior
/// is documented here.
///
///   * **Method**
///
///     Extracts the [Method](/rocket/http/enum.Method.html) from the incoming
///     request.
///
///     _This implementation always returns successfully._
///
///   * **&URI**
///
///     Extracts the [`Uri`](/rocket/http/uri/struct.Uri.html) from the incoming
///     request.
///
///     _This implementation always returns successfully._
///
///   * **&Route**
///
///     Extracts the [Route](/rocket/struct.Route.html) from the request if one
///     is available. If a route is not available, the request is forwarded.
///
///     For information on when an `&Route` is available, see the
///     [`Request::route`](/rocket/struct.Request.html#method.route)
///     documentation.
///
///   * **Cookies**
///
///     Returns a borrow to the [Cookies](/rocket/http/enum.Cookies.html) in
///     the incoming request. Note that `Cookies` implements internal
///     mutability, so a handle to `Cookies` allows you to get _and_ set cookies
///     in the request.
///
///     _This implementation always returns successfully._
///
///   * **ContentType**
///
///     Extracts the [ContentType](/rocket/http/struct.ContentType.html) from
///     the incoming request. If the request didn't specify a Content-Type, the
///     request is forwarded.
///
///   * **SocketAddr**
///
///     Extracts the remote address of the incoming request as a `SocketAddr`.
///     If the remote address is not known, the request is forwarded.
///
///     _This implementation always returns successfully._
///
///   * **Option&lt;T>** _where_ **T: FromRequest**
///
///     The type `T` is derived from the incoming request using `T`'s
///     `FromRequest` implementation. If the derivation is a `Success`, the
///     derived value is returned in `Some`. Otherwise, a `None` is returned.
///
///     _This implementation always returns successfully._
///
///   * **Result&lt;T, T::Error>** _where_ **T: FromRequest**
///
///     The type `T` is derived from the incoming request using `T`'s
///     `FromRequest` implementation. If derivation is a `Success`, the value is
///     returned in `Ok`. If the derivation is a `Failure`, the error value is
///     returned in `Err`. If the derivation is a `Forward`, the request is
///     forwarded.
///
/// # Example
///
/// Imagine you're running an authenticated API service that requires that some
/// requests be sent along with a valid API key in a header field. You want to
/// ensure that the handlers corresponding to these requests don't get called
/// unless there is an API key in the request and the key is valid. The
/// following example implements this using an `ApiKey` type and a `FromRequest`
/// implementation for that type. The `ApiKey` type is then used in the
/// `sensitive` handler.
///
/// ```rust
/// # #![feature(plugin, decl_macro)]
/// # #![plugin(rocket_codegen)]
/// # extern crate rocket;
/// #
/// use rocket::Outcome;
/// use rocket::http::Status;
/// use rocket::request::{self, Request, FromRequest};
///
/// struct ApiKey(String);
///
/// /// Returns true if `key` is a valid API key string.
/// fn is_valid(key: &str) -> bool {
///     key == "valid_api_key"
/// }
///
/// impl<'a, 'r> FromRequest<'a, 'r> for ApiKey {
///     type Error = ();
///
///     fn from_request(request: &'a Request<'r>) -> request::Outcome<ApiKey, ()> {
///         let keys: Vec<_> = request.headers().get("x-api-key").collect();
///         if keys.len() != 1 {
///             return Outcome::Failure((Status::BadRequest, ()));
///         }
///
///         let key = keys[0];
///         if !is_valid(keys[0]) {
///             return Outcome::Forward(());
///         }
///
///         return Outcome::Success(ApiKey(key.to_string()));
///     }
/// }
///
/// #[get("/sensitive")]
/// fn sensitive(key: ApiKey) -> &'static str {
/// #   let _key = key;
///     "Sensitive data."
/// }
///
/// # fn main() { }
/// ```
pub trait FromRequest<'a, 'r>: Sized {
    /// The associated error to be returned if derivation fails.
    type Error: Debug;

    /// Derives an instance of `Self` from the incoming request metadata.
    ///
    /// If the derivation is successful, an outcome of `Success` is returned. If
    /// the derivation fails in an unrecoverable fashion, `Failure` is returned.
    /// `Forward` is returned to indicate that the request should be forwarded
    /// to other matching routes, if any.
    fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error>;
}

impl<'a, 'r> FromRequest<'a, 'r> for Method {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        Success(request.method())
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for &'a Uri<'a> {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        Success(request.uri())
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for &'r Route {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        match request.route() {
            Some(route) => Success(route),
            None => Forward(())
        }
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for Cookies<'a> {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        Success(request.cookies())
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for &'a Accept {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        match request.accept() {
            Some(accept) => Success(accept),
            None => Forward(())
        }
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for &'a ContentType {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        match request.content_type() {
            Some(content_type) => Success(content_type),
            None => Forward(())
        }
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for SocketAddr {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        match request.remote() {
            Some(addr) => Success(addr),
            None => Forward(())
        }
    }
}

impl<'a, 'r, T: FromRequest<'a, 'r>> FromRequest<'a, 'r> for Result<T, T::Error> {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        match T::from_request(request) {
            Success(val) => Success(Ok(val)),
            Failure((_, e)) => Success(Err(e)),
            Forward(_) => Forward(()),
        }
    }
}

impl<'a, 'r, T: FromRequest<'a, 'r>> FromRequest<'a, 'r> for Option<T> {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        match T::from_request(request) {
            Success(val) => Success(Some(val)),
            Failure(_) | Forward(_) => Success(None),
        }
    }
}

