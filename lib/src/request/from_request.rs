use std::fmt::Debug;
use std::net::SocketAddr;

use outcome::{self, IntoOutcome};
use request::Request;
use outcome::Outcome::*;

use http::{Status, ContentType, Method, Cookies};
use http::uri::URI;

/// Type alias for the `Outcome` of a `FromRequest` conversion.
pub type Outcome<S, E> = outcome::Outcome<S, (Status, E), ()>;

impl<S, E> IntoOutcome<S, (Status, E), ()> for Result<S, E> {
    fn into_outcome(self) -> Outcome<S, E> {
        match self {
            Ok(val) => Success(val),
            Err(val) => Failure((Status::BadRequest, val))
        }
    }
}

/// Trait used to derive an object from incoming request metadata.
///
/// An arbitrary number of types that implement this trait can appear as
/// parameters in a route handler, as illustrated below:
///
/// ```rust,ignore
/// #[get("/")]
/// fn index(a: A, b: B, c: C) -> ... { ... }
/// ```
///
/// In this example, `A`, `B`, and `C` can be any types that implements
/// `FromRequest`. There can be any number of `FromRequest` types in the
/// function signature. Note that unlike every other derived object in Rocket,
/// `FromRequest` parameter names do not need to be declared in the route
/// attribute.
///
/// Derivation of `FromRequest` arguments is always attemped in left-to-right
/// declaration order. In the example above, for instance, the order will be `a`
/// followed by `b` followed by `c`. If a deriviation fails, the following
/// aren't attempted.
///
/// # Outcomes
///
/// The returned [Outcome](/rocket/outcome/index.html) of a `from_request` call
/// determines how the incoming request will be processed.
///
/// * **Success**(S)
///
///   If the `Outcome` is `Success`, then the `Success` value will be used as
///   the value for the corresponding parameter.  As long as all other parsed
///   types succeed, the request will be handled.
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
///   * **URI**
///
///     Extracts the [URI](/rocket/http/uri/struct.URI.html) from the incoming
///     request.
///
///     _This implementation always returns successfully._
///
///   * **Method**
///
///     Extracts the [Method](/rocket/http/enum.Method.html) from the incoming
///     request.
///
///     _This implementation always returns successfully._
///
///   * **&Cookies**
///
///     Returns a borrow to the [Cookies](/rocket/http/type.Cookies.html) in the
///     incoming request. Note that `Cookies` implements internal mutability, so
///     a handle to `&Cookies` allows you to get _and_ set cookies in the
///     request.
///
///     _This implementation always returns successfully._
///
///   * **ContentType**
///
///     Extracts the [ContentType](/rocket/http/struct.ContentType.html) from
///     the incoming request. If the request didn't specify a Content-Type, a
///     Content-Type of `*/*` (`Any`) is returned.
///
///     _This implementation always returns successfully._
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
///     dervived value is returned in `Some`. Otherwise, a `None` is returned.
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
/// following example implements this using an `APIKey` type and a `FromRequest`
/// implementation for that type. The `APIKey` type is then used in the
/// `senstive` handler.
///
/// ```rust
/// # #![feature(plugin)]
/// # #![plugin(rocket_codegen)]
/// # extern crate rocket;
/// #
/// use rocket::Outcome;
/// use rocket::http::Status;
/// use rocket::request::{self, Request, FromRequest};
///
/// struct APIKey(String);
///
/// /// Returns true if `key` is a valid API key string.
/// fn is_valid(key: &str) -> bool {
///     key == "valid_api_key"
/// }
///
/// impl<'a, 'r> FromRequest<'a, 'r> for APIKey {
///     type Error = ();
///
///     fn from_request(request: &'a Request<'r>) -> request::Outcome<APIKey, ()> {
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
///         return Outcome::Success(APIKey(key.to_string()));
///     }
/// }
///
/// #[get("/sensitive")]
/// fn sensitive(key: APIKey) -> &'static str {
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

impl<'a, 'r> FromRequest<'a, 'r> for &'a URI<'a> {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        Success(request.uri())
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for Method {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        Success(request.method())
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for &'a Cookies {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        Success(request.cookies())
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for ContentType {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        Success(request.content_type())
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

