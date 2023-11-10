//! Success, error, and forward handling.
//!
//! The `Outcome<S, E, F>` type is similar to the standard library's `Result<S,
//! E>` type. It is an enum with three variants, each containing a value:
//! `Success(S)`, which represents a successful outcome, `Error(E)`, which
//! represents an erroring outcome, and `Forward(F)`, which represents neither a
//! success or error, but instead, indicates that processing could not be
//! handled and should instead be _forwarded_ to whatever can handle the
//! processing next.
//!
//! The `Outcome` type is the return type of many of the core Rocket traits,
//! including [`FromRequest`](crate::request::FromRequest), [`FromData`]
//! [`Responder`]. It is also the return type of request handlers via the
//! [`Response`](crate::response::Response) type.
//!
//! [`FromData`]: crate::data::FromData
//! [`Responder`]: crate::response::Responder
//!
//! # Success
//!
//! A successful `Outcome<S, E, F>`, `Success(S)`, is returned from functions
//! that complete successfully. The meaning of a `Success` outcome depends on
//! the context. For instance, the `Outcome` of the `from_data` method of the
//! [`FromData`] trait will be matched against the type expected by
//! the user. For example, consider the following handler:
//!
//! ```rust
//! # use rocket::post;
//! # type S = String;
//! #[post("/", data = "<my_val>")]
//! fn hello(my_val: S) { /* ... */  }
//! ```
//!
//! The [`FromData`] implementation for the type `S` returns an `Outcome` with a
//! `Success(S)`. If `from_data` returns a `Success`, the `Success` value will
//! be unwrapped and the value will be used as the value of `my_val`.
//!
//! # Error
//!
//! An error `Outcome<S, E, F>`, `Error(E)`, is returned when a function
//! fails with some error and no processing can or should continue as a result.
//! The meaning of an error depends on the context.
//!
//! In Rocket, an `Error` generally means that a request is taken out of normal
//! processing. The request is then given to the catcher corresponding to some
//! status code. Users can catch errors by requesting a type of `Result<S, E>`
//! or `Option<S>` in request handlers. For example, if a user's handler looks
//! like:
//!
//! ```rust
//! # use rocket::post;
//! # type S = Option<String>;
//! # type E = std::convert::Infallible;
//! #[post("/", data = "<my_val>")]
//! fn hello(my_val: Result<S, E>) { /* ... */ }
//! ```
//!
//! The [`FromData`] implementation for the type `S` returns an `Outcome` with a
//! `Success(S)` and `Error(E)`. If `from_data` returns an `Error`, the `Error`
//! value will be unwrapped and the value will be used as the `Err` value of
//! `my_val` while a `Success` will be unwrapped and used the `Ok` value.
//!
//! # Forward
//!
//! A forward `Outcome<S, E, F>`, `Forward(F)`, is returned when a function
//! wants to indicate that the requested processing should be _forwarded_ to the
//! next available processor. Again, the exact meaning depends on the context.
//!
//! In Rocket, a `Forward` generally means that a request is forwarded to the
//! next available request handler. For example, consider the following request
//! handler:
//!
//! ```rust
//! # use rocket::post;
//! # type S = String;
//! #[post("/", data = "<my_val>")]
//! fn hello(my_val: S) { /* ... */ }
//! ```
//!
//! The [`FromData`] implementation for the type `S` returns an `Outcome` with a
//! `Success(S)`, `Error(E)`, and `Forward(F)`. If the `Outcome` is a
//! `Forward`, the `hello` handler isn't called. Instead, the incoming request
//! is forwarded, or passed on to, the next matching route, if any. Ultimately,
//! if there are no non-forwarding routes, forwarded requests are handled by the
//! 404 catcher. Similar to `Error`s, users can catch `Forward`s by requesting
//! a type of `Option<S>`. If an `Outcome` is a `Forward`, the `Option` will be
//! `None`.

use std::fmt;

use yansi::{Paint, Color};

use crate::{route, request, response};
use crate::data::{self, Data, FromData};
use crate::http::Status;

use self::Outcome::*;

/// An enum representing success (`Success`), error (`Error`), or forwarding
/// (`Forward`).
///
/// See the [top level documentation](crate::outcome) for detailed information.
#[must_use]
#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum Outcome<S, E, F> {
    /// Contains the success value.
    Success(S),
    /// Contains the error error value.
    Error(E),
    /// Contains the value to forward on.
    Forward(F),
}

impl<S, E, F> Outcome<S, E, F> {
    /// Unwraps the Outcome, yielding the contents of a Success.
    ///
    /// # Panics
    ///
    /// Panics if the value is not `Success`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use rocket::outcome::Outcome;
    /// # use rocket::outcome::Outcome::*;
    /// #
    /// let x: Outcome<i32, &str, usize> = Success(10);
    /// assert_eq!(x.unwrap(), 10);
    /// ```
    #[inline]
    #[track_caller]
    pub fn unwrap(self) -> S {
        match self {
            Success(val) => val,
            _ => panic!("unwrapped a non-successful outcome")
        }
    }

    /// Unwraps the Outcome, yielding the contents of a Success.
    ///
    /// # Panics
    ///
    /// If the value is not `Success`, panics with the given `message`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use rocket::outcome::Outcome;
    /// # use rocket::outcome::Outcome::*;
    /// #
    /// let x: Outcome<i32, &str, usize> = Success(10);
    /// assert_eq!(x.expect("success value"), 10);
    /// ```
    #[inline]
    #[track_caller]
    pub fn expect(self, message: &str) -> S {
        match self {
            Success(val) => val,
            _ => panic!("unwrapped a non-successful outcome: {}", message)
        }
    }

    /// Return true if this `Outcome` is a `Success`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use rocket::outcome::Outcome;
    /// # use rocket::outcome::Outcome::*;
    /// #
    /// let x: Outcome<i32, &str, usize> = Success(10);
    /// assert_eq!(x.is_success(), true);
    ///
    /// let x: Outcome<i32, &str, usize> = Error("Hi! I'm an error.");
    /// assert_eq!(x.is_success(), false);
    ///
    /// let x: Outcome<i32, &str, usize> = Forward(25);
    /// assert_eq!(x.is_success(), false);
    /// ```
    #[inline]
    pub fn is_success(&self) -> bool {
        matches!(self, Success(_))
    }

    /// Return true if this `Outcome` is an `Error`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use rocket::outcome::Outcome;
    /// # use rocket::outcome::Outcome::*;
    /// #
    /// let x: Outcome<i32, &str, usize> = Success(10);
    /// assert_eq!(x.is_error(), false);
    ///
    /// let x: Outcome<i32, &str, usize> = Error("Hi! I'm an error.");
    /// assert_eq!(x.is_error(), true);
    ///
    /// let x: Outcome<i32, &str, usize> = Forward(25);
    /// assert_eq!(x.is_error(), false);
    /// ```
    #[inline]
    pub fn is_error(&self) -> bool {
        matches!(self, Error(_))
    }

    /// Return true if this `Outcome` is a `Forward`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use rocket::outcome::Outcome;
    /// # use rocket::outcome::Outcome::*;
    /// #
    /// let x: Outcome<i32, &str, usize> = Success(10);
    /// assert_eq!(x.is_forward(), false);
    ///
    /// let x: Outcome<i32, &str, usize> = Error("Hi! I'm an error.");
    /// assert_eq!(x.is_forward(), false);
    ///
    /// let x: Outcome<i32, &str, usize> = Forward(25);
    /// assert_eq!(x.is_forward(), true);
    /// ```
    #[inline]
    pub fn is_forward(&self) -> bool {
        matches!(self, Forward(_))
    }

    /// Converts from `Outcome<S, E, F>` to `Option<S>`.
    ///
    /// Returns the `Some` of the `Success` if this is a `Success`, otherwise
    /// returns `None`. `self` is consumed, and all other values are discarded.
    ///
    /// ```rust
    /// # use rocket::outcome::Outcome;
    /// # use rocket::outcome::Outcome::*;
    /// #
    /// let x: Outcome<i32, &str, usize> = Success(10);
    /// assert_eq!(x.succeeded(), Some(10));
    ///
    /// let x: Outcome<i32, &str, usize> = Error("Hi! I'm an error.");
    /// assert_eq!(x.succeeded(), None);
    ///
    /// let x: Outcome<i32, &str, usize> = Forward(25);
    /// assert_eq!(x.succeeded(), None);
    /// ```
    #[inline]
    pub fn succeeded(self) -> Option<S> {
        match self {
            Success(val) => Some(val),
            _ => None
        }
    }

    /// Converts from `Outcome<S, E, F>` to `Option<E>`.
    ///
    /// Returns the `Some` of the `Error` if this is an `Error`, otherwise
    /// returns `None`. `self` is consumed, and all other values are discarded.
    ///
    /// ```rust
    /// # use rocket::outcome::Outcome;
    /// # use rocket::outcome::Outcome::*;
    /// #
    /// let x: Outcome<i32, &str, usize> = Success(10);
    /// assert_eq!(x.failed(), None);
    ///
    /// let x: Outcome<i32, &str, usize> = Error("Hi! I'm an error.");
    /// assert_eq!(x.failed(), Some("Hi! I'm an error."));
    ///
    /// let x: Outcome<i32, &str, usize> = Forward(25);
    /// assert_eq!(x.failed(), None);
    /// ```
    #[inline]
    pub fn failed(self) -> Option<E> {
        match self {
            Error(val) => Some(val),
            _ => None
        }
    }

    /// Converts from `Outcome<S, E, F>` to `Option<F>`.
    ///
    /// Returns the `Some` of the `Forward` if this is a `Forward`, otherwise
    /// returns `None`. `self` is consumed, and all other values are discarded.
    ///
    /// ```rust
    /// # use rocket::outcome::Outcome;
    /// # use rocket::outcome::Outcome::*;
    /// #
    /// let x: Outcome<i32, &str, usize> = Success(10);
    /// assert_eq!(x.forwarded(), None);
    ///
    /// let x: Outcome<i32, &str, usize> = Error("Hi! I'm an error.");
    /// assert_eq!(x.forwarded(), None);
    ///
    /// let x: Outcome<i32, &str, usize> = Forward(25);
    /// assert_eq!(x.forwarded(), Some(25));
    /// ```
    #[inline]
    pub fn forwarded(self) -> Option<F> {
        match self {
            Forward(val) => Some(val),
            _ => None
        }
    }

    /// Returns a `Success` value as `Ok()` or `value` in `Err`. Converts from
    /// `Outcome<S, E, F>` to `Result<S, T>` for a given `T`.
    ///
    /// Returns `Ok` with the `Success` value if this is a `Success`, otherwise
    /// returns an `Err` with the provided value. `self` is consumed, and all
    /// other values are discarded.
    ///
    /// ```rust
    /// # use rocket::outcome::Outcome;
    /// # use rocket::outcome::Outcome::*;
    /// #
    /// let x: Outcome<i32, &str, usize> = Success(10);
    /// assert_eq!(x.success_or(false), Ok(10));
    ///
    /// let x: Outcome<i32, &str, usize> = Error("Hi! I'm an error.");
    /// assert_eq!(x.success_or(false), Err(false));
    ///
    /// let x: Outcome<i32, &str, usize> = Forward(25);
    /// assert_eq!(x.success_or("whoops"), Err("whoops"));
    /// ```
    #[inline]
    pub fn success_or<T>(self, value: T) -> Result<S, T> {
        match self {
            Success(val) => Ok(val),
            _ => Err(value)
        }
    }

    /// Returns a `Success` value as `Ok()` or `f()` in `Err`. Converts from
    /// `Outcome<S, E, F>` to `Result<S, T>` for a given `T` produced from a
    /// supplied function or closure.
    ///
    /// Returns `Ok` with the `Success` value if this is a `Success`, otherwise
    /// returns an `Err` with the result of calling `f`. `self` is consumed, and
    /// all other values are discarded.
    ///
    /// ```rust
    /// # use rocket::outcome::Outcome;
    /// # use rocket::outcome::Outcome::*;
    /// #
    /// let x: Outcome<i32, &str, usize> = Success(10);
    /// assert_eq!(x.success_or_else(|| false), Ok(10));
    ///
    /// let x: Outcome<i32, &str, usize> = Error("Hi! I'm an error.");
    /// assert_eq!(x.success_or_else(|| false), Err(false));
    ///
    /// let x: Outcome<i32, &str, usize> = Forward(25);
    /// assert_eq!(x.success_or_else(|| "whoops"), Err("whoops"));
    /// ```
    #[inline]
    pub fn success_or_else<T, V: FnOnce() -> T>(self, f: V) -> Result<S, T> {
        match self {
            Success(val) => Ok(val),
            _ => Err(f())
        }
    }

    /// Converts from `Outcome<S, E, F>` to `Outcome<&S, &E, &F>`.
    ///
    /// ```rust
    /// # use rocket::outcome::Outcome;
    /// # use rocket::outcome::Outcome::*;
    /// #
    /// let x: Outcome<i32, &str, usize> = Success(10);
    /// assert_eq!(x.as_ref(), Success(&10));
    ///
    /// let x: Outcome<i32, &str, usize> = Error("Hi! I'm an error.");
    /// assert_eq!(x.as_ref(), Error(&"Hi! I'm an error."));
    /// ```
    #[inline]
    pub fn as_ref(&self) -> Outcome<&S, &E, &F> {
        match *self {
            Success(ref val) => Success(val),
            Error(ref val) => Error(val),
            Forward(ref val) => Forward(val),
        }
    }

    /// Converts from `Outcome<S, E, F>` to `Outcome<&mut S, &mut E, &mut F>`.
    ///
    /// ```rust
    /// # use rocket::outcome::Outcome;
    /// # use rocket::outcome::Outcome::*;
    /// #
    /// let mut x: Outcome<i32, &str, usize> = Success(10);
    /// if let Success(val) = x.as_mut() {
    ///     *val = 20;
    /// }
    ///
    /// assert_eq!(x.unwrap(), 20);
    /// ```
    #[inline]
    pub fn as_mut(&mut self) -> Outcome<&mut S, &mut E, &mut F> {
        match *self {
            Success(ref mut val) => Success(val),
            Error(ref mut val) => Error(val),
            Forward(ref mut val) => Forward(val),
        }
    }

    /// Maps the `Success` value using `f`. Maps an `Outcome<S, E, F>` to an
    /// `Outcome<T, E, F>` by applying the function `f` to the value of type `S`
    /// in `self` if `self` is an `Outcome::Success`.
    ///
    /// ```rust
    /// # use rocket::outcome::Outcome;
    /// # use rocket::outcome::Outcome::*;
    /// #
    /// let x: Outcome<i32, &str, usize> = Success(10);
    ///
    /// let mapped = x.map(|v| if v == 10 { "10" } else { "not 10" });
    /// assert_eq!(mapped, Success("10"));
    /// ```
    #[inline]
    pub fn map<T, M: FnOnce(S) -> T>(self, f: M) -> Outcome<T, E, F> {
        match self {
            Success(val) => Success(f(val)),
            Error(val) => Error(val),
            Forward(val) => Forward(val),
        }
    }

    /// Maps the `Error` value using `f`. Maps an `Outcome<S, E, F>` to an
    /// `Outcome<S, T, F>` by applying the function `f` to the value of type `E`
    /// in `self` if `self` is an `Outcome::Error`.
    ///
    /// ```rust
    /// # use rocket::outcome::Outcome;
    /// # use rocket::outcome::Outcome::*;
    /// #
    /// let x: Outcome<i32, &str, usize> = Error("hi");
    ///
    /// let mapped = x.map_error(|v| if v == "hi" { 10 } else { 0 });
    /// assert_eq!(mapped, Error(10));
    /// ```
    #[inline]
    pub fn map_error<T, M: FnOnce(E) -> T>(self, f: M) -> Outcome<S, T, F> {
        match self {
            Success(val) => Success(val),
            Error(val) => Error(f(val)),
            Forward(val) => Forward(val),
        }
    }

    /// Maps the `Forward` value using `f`. Maps an `Outcome<S, E, F>` to an
    /// `Outcome<S, E, T>` by applying the function `f` to the value of type `F`
    /// in `self` if `self` is an `Outcome::Forward`.
    ///
    /// ```rust
    /// # use rocket::outcome::Outcome;
    /// # use rocket::outcome::Outcome::*;
    /// #
    /// let x: Outcome<i32, &str, usize> = Forward(5);
    ///
    /// let mapped = x.map_forward(|v| if v == 5 { "a" } else { "b" });
    /// assert_eq!(mapped, Forward("a"));
    /// ```
    #[inline]
    pub fn map_forward<T, M: FnOnce(F) -> T>(self, f: M) -> Outcome<S, E, T> {
        match self {
            Success(val) => Success(val),
            Error(val) => Error(val),
            Forward(val) => Forward(f(val)),
        }
    }

    /// Converts from `Outcome<S, E, F>` to `Outcome<T, E, F>` using `f` to map
    /// `Success(S)` to `Success(T)`.
    ///
    /// If `self` is not `Success`, `self` is returned.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use rocket::outcome::Outcome;
    /// # use rocket::outcome::Outcome::*;
    /// #
    /// let x: Outcome<i32, &str, bool> = Success(10);
    ///
    /// let mapped = x.and_then(|v| match v {
    ///    10 => Success("10"),
    ///    1 => Forward(false),
    ///    _ => Error("30")
    /// });
    ///
    /// assert_eq!(mapped, Success("10"));
    /// ```
    #[inline]
    pub fn and_then<T, M: FnOnce(S) -> Outcome<T, E, F>>(self, f: M) -> Outcome<T, E, F> {
        match self {
            Success(val) => f(val),
            Error(val) => Error(val),
            Forward(val) => Forward(val),
        }
    }

    /// Converts from `Outcome<S, E, F>` to `Outcome<S, T, F>` using `f` to map
    /// `Error(E)` to `Error(T)`.
    ///
    /// If `self` is not `Error`, `self` is returned.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use rocket::outcome::Outcome;
    /// # use rocket::outcome::Outcome::*;
    /// #
    /// let x: Outcome<i32, &str, bool> = Error("hi");
    ///
    /// let mapped = x.error_then(|v| match v {
    ///    "hi" => Error(10),
    ///    "test" => Forward(false),
    ///    _ => Success(10)
    /// });
    ///
    /// assert_eq!(mapped, Error(10));
    /// ```
    #[inline]
    pub fn error_then<T, M: FnOnce(E) -> Outcome<S, T, F>>(self, f: M) -> Outcome<S, T, F> {
        match self {
            Success(val) => Success(val),
            Error(val) => f(val),
            Forward(val) => Forward(val),
        }
    }

    /// Converts from `Outcome<S, E, F>` to `Outcome<S, E, T>` using `f` to map
    /// `Forward(F)` to `Forward(T)`.
    ///
    /// If `self` is not `Forward`, `self` is returned.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use rocket::outcome::Outcome;
    /// # use rocket::outcome::Outcome::*;
    /// #
    /// let x: Outcome<i32, &str, Option<bool>> = Forward(Some(false));
    ///
    /// let mapped = x.forward_then(|v| match v {
    ///    Some(true) => Success(10),
    ///    Some(false) => Forward(20),
    ///    None => Error("10")
    /// });
    ///
    /// assert_eq!(mapped, Forward(20));
    /// ```
    #[inline]
    pub fn forward_then<T, M: FnOnce(F) -> Outcome<S, E, T>>(self, f: M) -> Outcome<S, E, T> {
        match self {
            Success(val) => Success(val),
            Error(val) => Error(val),
            Forward(val) => f(val),
        }
    }

    /// Converts `Outcome<S, E, F>` to `Result<S, E>` by identity mapping
    /// `Success(S)` and `Error(E)` to `Result<T, E>` and mapping `Forward(F)`
    /// to `Result<T, E>` using `f`.
    ///
    /// ```rust
    /// # use rocket::outcome::Outcome;
    /// # use rocket::outcome::Outcome::*;
    /// #
    /// let x: Outcome<i32, &str, usize> = Success(10);
    /// assert_eq!(x.ok_map_forward(|x| Ok(x as i32 + 1)), Ok(10));
    ///
    /// let x: Outcome<i32, &str, usize> = Error("hello");
    /// assert_eq!(x.ok_map_forward(|x| Ok(x as i32 + 1)), Err("hello"));
    ///
    /// let x: Outcome<i32, &str, usize> = Forward(0);
    /// assert_eq!(x.ok_map_forward(|x| Ok(x as i32 + 1)), Ok(1));
    /// ```
    #[inline]
    pub fn ok_map_forward<M>(self, f: M) -> Result<S, E>
        where M: FnOnce(F) -> Result<S, E>
    {
        match self {
            Outcome::Success(s) => Ok(s),
            Outcome::Error(e) => Err(e),
            Outcome::Forward(v) => f(v),
        }
    }

    /// Converts `Outcome<S, E, F>` to `Result<S, E>` by identity mapping
    /// `Success(S)` and `Forward(F)` to `Result<T, F>` and mapping `Error(E)`
    /// to `Result<T, F>` using `f`.
    ///
    /// ```rust
    /// # use rocket::outcome::Outcome;
    /// # use rocket::outcome::Outcome::*;
    /// #
    /// let x: Outcome<i32, &str, usize> = Success(10);
    /// assert_eq!(x.ok_map_error(|s| Ok(123)), Ok(10));
    ///
    /// let x: Outcome<i32, &str, usize> = Error("hello");
    /// assert_eq!(x.ok_map_error(|s| Ok(123)), Ok(123));
    ///
    /// let x: Outcome<i32, &str, usize> = Forward(0);
    /// assert_eq!(x.ok_map_error(|s| Ok(123)), Err(0));
    /// ```
    #[inline]
    pub fn ok_map_error<M>(self, f: M) -> Result<S, F>
        where M: FnOnce(E) -> Result<S, F>
    {
        match self {
            Outcome::Success(s) => Ok(s),
            Outcome::Error(e) => f(e),
            Outcome::Forward(v) => Err(v),
        }
    }
}

impl<'a, S: Send + 'a, E: Send + 'a, F: Send + 'a> Outcome<S, E, F> {
    /// Pins a future that resolves to `self`, returning a
    /// [`BoxFuture`](crate::futures::future::BoxFuture) that resolves to
    /// `self`.
    #[inline]
    pub fn pin(self) -> futures::future::BoxFuture<'a, Self> {
        Box::pin(async move { self })
    }
}

crate::export! {
    /// Unwraps a [`Success`](Outcome::Success) or propagates a `Forward` or
    /// `Error` by returning early.
    ///
    /// # Syntax
    ///
    /// The macro has the following "signature":
    ///
    /// ```rust
    /// use rocket::outcome::Outcome;
    ///
    /// // Returns the inner `S` if `outcome` is `Outcome::Success`. Otherwise
    /// // returns from the caller with `Outcome<impl From<E>, impl From<F>>`.
    /// fn try_outcome<S, E, F>(outcome: Outcome<S, E, F>) -> S
    /// # { unimplemented!() }
    /// ```
    ///
    /// This is just like `?` (or previously, `try!`), but for `Outcome`. In the
    /// case of a `Forward` or `Error` variant, the inner type is passed to
    /// [`From`](std::convert::From), allowing for the conversion between
    /// specific and more general types. The resulting forward/error is
    /// immediately returned. Because of the early return, `try_outcome!` can
    /// only be used in methods that return [`Outcome`].
    ///
    /// [`Outcome`]: crate::outcome::Outcome
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// # #[macro_use] extern crate rocket;
    /// use std::sync::atomic::{AtomicUsize, Ordering};
    ///
    /// use rocket::State;
    /// use rocket::request::{self, Request, FromRequest};
    /// use rocket::outcome::{try_outcome, Outcome::*};
    ///
    /// #[derive(Default)]
    /// struct Atomics {
    ///     uncached: AtomicUsize,
    ///     cached: AtomicUsize,
    /// }
    ///
    /// struct Guard1;
    /// struct Guard2;
    ///
    /// #[rocket::async_trait]
    /// impl<'r> FromRequest<'r> for Guard1 {
    ///     type Error = ();
    ///
    ///     async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, ()> {
    ///         // Attempt to fetch the guard, passing through any error or forward.
    ///         let atomics = try_outcome!(req.guard::<&State<Atomics>>().await);
    ///         atomics.uncached.fetch_add(1, Ordering::Relaxed);
    ///         req.local_cache(|| atomics.cached.fetch_add(1, Ordering::Relaxed));
    ///
    ///         Success(Guard1)
    ///     }
    /// }
    ///
    /// #[rocket::async_trait]
    /// impl<'r> FromRequest<'r> for Guard2 {
    ///     type Error = ();
    ///
    ///     async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, ()> {
    ///         // Attempt to fetch the guard, passing through any error or forward.
    ///         let guard1: Guard1 = try_outcome!(req.guard::<Guard1>().await);
    ///         Success(Guard2)
    ///     }
    /// }
    /// ```
    macro_rules! try_outcome {
        ($expr:expr $(,)?) => (match $expr {
            $crate::outcome::Outcome::Success(val) => val,
            $crate::outcome::Outcome::Error(e) => {
                return $crate::outcome::Outcome::Error(::std::convert::From::from(e))
            },
            $crate::outcome::Outcome::Forward(f) => {
                return $crate::outcome::Outcome::Forward(::std::convert::From::from(f))
            },
        });
    }
}

impl<S, E, F> Outcome<S, E, F> {
    #[inline]
    fn dbg_str(&self) -> &'static str {
        match self {
            Success(..) => "Success",
            Error(..) => "Error",
            Forward(..) => "Forward",
        }
    }

    #[inline]
    fn color(&self) -> Color {
        match self {
            Success(..) => Color::Green,
            Error(..) => Color::Red,
            Forward(..) => Color::Yellow,
        }
    }
}

pub(crate) struct Display<'a, 'r>(&'a route::Outcome<'r>);

impl<'r> route::Outcome<'r> {
    pub(crate) fn log_display(&self) -> Display<'_, 'r> {
        impl fmt::Display for Display<'_, '_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", "Outcome: ".primary().bold())?;

                let color = self.0.color();
                match self.0 {
                    Success(r) => write!(f, "{}({})", "Success".paint(color), r.status().primary()),
                    Error(s) => write!(f, "{}({})", "Error".paint(color), s.primary()),
                    Forward((_, s)) => write!(f, "{}({})", "Forward".paint(color), s.primary()),
                }
            }
        }

        Display(self)
    }
}

impl<S, E, F> fmt::Debug for Outcome<S, E, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Outcome::{}", self.dbg_str())
    }
}

impl<S, E, F> fmt::Display for Outcome<S, E, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.dbg_str().paint(self.color()))
    }
}

/// Conversion trait from some type into an Outcome type.
pub trait IntoOutcome<Outcome> {
    /// The type to use when returning an `Outcome::Error`.
    type Error: Sized;

    /// The type to use when returning an `Outcome::Forward`.
    type Forward: Sized;

    /// Converts `self` into an `Outcome`. If `self` represents a success, an
    /// `Outcome::Success` is returned. Otherwise, an `Outcome::Error` is
    /// returned with `error` as the inner value.
    fn or_error(self, error: Self::Error) -> Outcome;

    /// Converts `self` into an `Outcome`. If `self` represents a success, an
    /// `Outcome::Success` is returned. Otherwise, an `Outcome::Forward` is
    /// returned with `forward` as the inner value.
    fn or_forward(self, forward: Self::Forward) -> Outcome;
}

impl<S, E, F> IntoOutcome<Outcome<S, E, F>> for Option<S> {
    type Error = E;
    type Forward = F;

    #[inline]
    fn or_error(self, error: E) -> Outcome<S, E, F> {
        match self {
            Some(val) => Success(val),
            None => Error(error)
        }
    }

    #[inline]
    fn or_forward(self, forward: F) -> Outcome<S, E, F> {
        match self {
            Some(val) => Success(val),
            None => Forward(forward)
        }
    }
}

impl<'r, T: FromData<'r>> IntoOutcome<data::Outcome<'r, T>> for Result<T, T::Error> {
    type Error = Status;
    type Forward = (Data<'r>, Status);

    #[inline]
    fn or_error(self, error: Status) -> data::Outcome<'r, T> {
        match self {
            Ok(val) => Success(val),
            Err(err) => Error((error, err))
        }
    }

    #[inline]
    fn or_forward(self, (data, forward): (Data<'r>, Status)) -> data::Outcome<'r, T> {
        match self {
            Ok(val) => Success(val),
            Err(_) => Forward((data, forward))
        }
    }
}

impl<S, E> IntoOutcome<request::Outcome<S, E>> for Result<S, E> {
    type Error = Status;
    type Forward = Status;

    #[inline]
    fn or_error(self, error: Status) -> request::Outcome<S, E> {
        match self {
            Ok(val) => Success(val),
            Err(err) => Error((error, err))
        }
    }

    #[inline]
    fn or_forward(self, status: Status) -> request::Outcome<S, E> {
        match self {
            Ok(val) => Success(val),
            Err(_) => Forward(status)
        }
    }
}

impl<'r, 'o: 'r> IntoOutcome<route::Outcome<'r>> for response::Result<'o> {
    type Error = ();
    type Forward = (Data<'r>, Status);

    #[inline]
    fn or_error(self, _: ()) -> route::Outcome<'r> {
        match self {
            Ok(val) => Success(val),
            Err(status) => Error(status),
        }
    }

    #[inline]
    fn or_forward(self, (data, forward): (Data<'r>, Status)) -> route::Outcome<'r> {
        match self {
            Ok(val) => Success(val),
            Err(_) => Forward((data, forward))
        }
    }
}
