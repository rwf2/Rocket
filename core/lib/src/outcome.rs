//! Success, failure, and forward handling.
//!
//! The `Outcome<S, E, F>` type is similar to the standard library's `Result<S,
//! E>` type. It is an enum with three variants, each containing a value:
//! `Success(S)`, which represents a successful outcome, `Failure(E)`, which
//! represents a failing outcome, and `Forward(F)`, which represents neither a
//! success or failure, but instead, indicates that processing could not be
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
//! [`FromData`] trait will be matched against the type expected by the user.
//! For example, consider the following handler:
//!
//! ```rust,ignore
//! #[post("/", data = "<my_val>")]
//! fn hello(my_val: S) -> ... {  }
//! ```
//!
//! The [`FromData`] implementation for the type `S` returns an `Outcome` with a
//! `Success(S)`. If `from_data` returns a `Success`, the `Success` value will
//! be unwrapped and the value will be used as the value of `my_val`.
//!
//! # Failure
//!
//! A failure `Outcome<S, E, F>`, `Failure(E)`, is returned when a function
//! fails with some error and no processing can or should continue as a result.
//! The meaning of a failure depends on the context.
//!
//! In Rocket, a `Failure` generally means that a request is taken out of normal
//! processing. The request is then given to the catcher corresponding to some
//! status code. Users can catch failures by requesting a type of `Result<S, E>`
//! or `Option<S>` in request handlers. For example, if a user's handler looks
//! like:
//!
//! ```rust,ignore
//! #[post("/", data = "<my_val>")]
//! fn hello(my_val: Result<S, E>) -> ... {  }
//! ```
//!
//! The [`FromData`] implementation for the type `S` returns an `Outcome` with a
//! `Success(S)` and `Failure(E)`. If `from_data` returns a `Failure`, the
//! `Failure` value will be unwrapped and the value will be used as the `Err`
//! value of `my_val` while a `Success` will be unwrapped and used the `Ok`
//! value.
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
//! ```rust,ignore
//! #[post("/", data = "<my_val>")]
//! fn hello(my_val: S) -> ... {  }
//! ```
//!
//! The [`FromData`] implementation for the type `S` returns an `Outcome` with a
//! `Success(S)`, `Failure(E)`, and `Forward(F)`. If the `Outcome` is a
//! `Forward`, the `hello` handler isn't called. Instead, the incoming request
//! is forwarded, or passed on to, the next matching route, if any. Ultimately,
//! if there are no non-forwarding routes, forwarded requests are handled by the
//! 404 catcher. Similar to `Failure`s, users can catch `Forward`s by requesting
//! a type of `Option<S>`. If an `Outcome` is a `Forward`, the `Option` will be
//! `None`.

use std::fmt;
use std::ops::{Try, ControlFlow, FromResidual};

use yansi::{Paint, Color};

use self::Outcome::*;

/// An enum representing success (`Success`), failure (`Failure`), or
/// forwarding (`Forward`).
///
/// See the [top level documentation](::outcome) for detailed information.
#[must_use]
#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum Outcome<S, E, F> {
    /// Contains the success value.
    Success(S),
    /// Contains the failure error value.
    Failure(E),
    /// Contains the value to forward on.
    Forward(F),
}

/// Conversion trait from some type into an Outcome type.
pub trait IntoOutcome<S, E, F> {
    /// The type to use when returning an `Outcome::Failure`.
    type Failure: Sized;

    /// The type to use when returning an `Outcome::Forward`.
    type Forward: Sized;

    /// Converts `self` into an `Outcome`. If `self` represents a success, an
    /// `Outcome::Success` is returned. Otherwise, an `Outcome::Failure` is
    /// returned with `failure` as the inner value.
    fn into_outcome(self, failure: Self::Failure) -> Outcome<S, E, F>;

    /// Converts `self` into an `Outcome`. If `self` represents a success, an
    /// `Outcome::Success` is returned. Otherwise, an `Outcome::Forward` is
    /// returned with `forward` as the inner value.
    fn or_forward(self, forward: Self::Forward) -> Outcome<S, E, F>;
}

impl<S, E, F> IntoOutcome<S, E, F> for Option<S> {
    type Failure = E;
    type Forward = F;

    #[inline]
    fn into_outcome(self, failure: E) -> Outcome<S, E, F> {
        match self {
            Some(val) => Success(val),
            None => Failure(failure)
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
    pub fn unwrap(self) -> S {
        match self {
            Success(val) => val,
            _ => panic!("Expected a successful outcome!")
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
    pub fn expect(self, message: &str) -> S {
        match self {
            Success(val) => val,
            _ => panic!("Outcome::expect() failed: {}", message)
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
    /// let x: Outcome<i32, &str, usize> = Failure("Hi! I'm an error.");
    /// assert_eq!(x.is_success(), false);
    ///
    /// let x: Outcome<i32, &str, usize> = Forward(25);
    /// assert_eq!(x.is_success(), false);
    /// ```
    #[inline]
    pub fn is_success(&self) -> bool {
        match *self {
            Success(_) => true,
            _ => false
        }
    }

    /// Return true if this `Outcome` is a `Failure`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use rocket::outcome::Outcome;
    /// # use rocket::outcome::Outcome::*;
    /// #
    /// let x: Outcome<i32, &str, usize> = Success(10);
    /// assert_eq!(x.is_failure(), false);
    ///
    /// let x: Outcome<i32, &str, usize> = Failure("Hi! I'm an error.");
    /// assert_eq!(x.is_failure(), true);
    ///
    /// let x: Outcome<i32, &str, usize> = Forward(25);
    /// assert_eq!(x.is_failure(), false);
    /// ```
    #[inline]
    pub fn is_failure(&self) -> bool {
        match *self {
            Failure(_) => true,
            _ => false
        }
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
    /// let x: Outcome<i32, &str, usize> = Failure("Hi! I'm an error.");
    /// assert_eq!(x.is_forward(), false);
    ///
    /// let x: Outcome<i32, &str, usize> = Forward(25);
    /// assert_eq!(x.is_forward(), true);
    /// ```
    #[inline]
    pub fn is_forward(&self) -> bool {
        match *self {
            Forward(_) => true,
            _ => false
        }
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
    /// let x: Outcome<i32, &str, usize> = Failure("Hi! I'm an error.");
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
    /// Returns the `Some` of the `Failure` if this is a `Failure`, otherwise
    /// returns `None`. `self` is consumed, and all other values are discarded.
    ///
    /// ```rust
    /// # use rocket::outcome::Outcome;
    /// # use rocket::outcome::Outcome::*;
    /// #
    /// let x: Outcome<i32, &str, usize> = Success(10);
    /// assert_eq!(x.failed(), None);
    ///
    /// let x: Outcome<i32, &str, usize> = Failure("Hi! I'm an error.");
    /// assert_eq!(x.failed(), Some("Hi! I'm an error."));
    ///
    /// let x: Outcome<i32, &str, usize> = Forward(25);
    /// assert_eq!(x.failed(), None);
    /// ```
    #[inline]
    pub fn failed(self) -> Option<E> {
        match self {
            Failure(val) => Some(val),
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
    /// let x: Outcome<i32, &str, usize> = Failure("Hi! I'm an error.");
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

    /// Converts from `Outcome<S, E, F>` to `Result<S, T>` for a given `T`.
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
    /// let x: Outcome<i32, &str, usize> = Failure("Hi! I'm an error.");
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

    /// Converts from `Outcome<S, E, F>` to `Result<S, T>` for a given `T`
    /// produced from a supplied function or closure.
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
    /// let x: Outcome<i32, &str, usize> = Failure("Hi! I'm an error.");
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
    /// let x: Outcome<i32, &str, usize> = Failure("Hi! I'm an error.");
    /// assert_eq!(x.as_ref(), Failure(&"Hi! I'm an error."));
    /// ```
    #[inline]
    pub fn as_ref(&self) -> Outcome<&S, &E, &F> {
        match *self {
            Success(ref val) => Success(val),
            Failure(ref val) => Failure(val),
            Forward(ref val) => Forward(val),
        }
    }

    /// Maps an `Outcome<S, E, F>` to an `Outcome<T, E, F>` by applying the
    /// function `f` to the value of type `S` in `self` if `self` is an
    /// `Outcome::Success`.
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
            Failure(val) => Failure(val),
            Forward(val) => Forward(val),
        }
    }

    /// Maps an `Outcome<S, E, F>` to an `Outcome<S, T, F>` by applying the
    /// function `f` to the value of type `E` in `self` if `self` is an
    /// `Outcome::Failure`.
    ///
    /// ```rust
    /// # use rocket::outcome::Outcome;
    /// # use rocket::outcome::Outcome::*;
    /// #
    /// let x: Outcome<i32, &str, usize> = Failure("hi");
    ///
    /// let mapped = x.map_failure(|v| if v == "hi" { 10 } else { 0 });
    /// assert_eq!(mapped, Failure(10));
    /// ```
    #[inline]
    pub fn map_failure<T, M: FnOnce(E) -> T>(self, f: M) -> Outcome<S, T, F> {
        match self {
            Success(val) => Success(val),
            Failure(val) => Failure(f(val)),
            Forward(val) => Forward(val),
        }
    }

    /// Maps an `Outcome<S, E, F>` to an `Outcome<S, E, T>` by applying the
    /// function `f` to the value of type `F` in `self` if `self` is an
    /// `Outcome::Forward`.
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
            Failure(val) => Failure(val),
            Forward(val) => Forward(f(val)),
        }
    }

    /// Maps an `Outcome<S, E, F>` to an `Outcome<T, E, F>` by applying the
    /// function `f` to the value of type `S` in `self` if `self` is an
    /// `Outcome::Success`.
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
    ///    _ => Failure("30")
    /// });
    ///
    /// assert_eq!(mapped, Success("10"));
    /// ```
    #[inline]
    pub fn and_then<T, M: FnOnce(S) -> Outcome<T, E, F>>(self, f: M) -> Outcome<T, E, F> {
        match self {
            Success(val) => f(val),
            Failure(val) => Failure(val),
            Forward(val) => Forward(val),
        }
    }

    /// Maps an `Outcome<S, E, F>` to an `Outcome<S, T, F>` by applying the
    /// function `f` to the value of type `E` in `self` if `self` is an
    /// `Outcome::Failure`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use rocket::outcome::Outcome;
    /// # use rocket::outcome::Outcome::*;
    /// #
    /// let x: Outcome<i32, &str, bool> = Failure("hi");
    ///
    /// let mapped = x.failure_then(|v| match v {
    ///    "hi" => Failure(10),
    ///    "test" => Forward(false),
    ///    _ => Success(10)
    /// });
    ///
    /// assert_eq!(mapped, Failure(10));
    /// ```
    #[inline]
    pub fn failure_then<T, M: FnOnce(E) -> Outcome<S, T, F>>(self, f: M) -> Outcome<S, T, F> {
        match self {
            Success(val) => Success(val),
            Failure(val) => f(val),
            Forward(val) => Forward(val),
        }
    }

    /// Maps an `Outcome<S, E, F>` to an `Outcome<S, E, T>` by applying the
    /// function `f` to the value of type `F` in `self` if `self` is an
    /// `Outcome::Forward`.
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
    ///    None => Failure("10")
    /// });
    ///
    /// assert_eq!(mapped, Forward(20));
    /// ```
    #[inline]
    pub fn forward_then<T, M: FnOnce(F) -> Outcome<S, E, T>>(self, f: M) -> Outcome<S, E, T> {
        match self {
            Success(val) => Success(val),
            Failure(val) => Failure(val),
            Forward(val) => f(val),
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
            Failure(ref mut val) => Failure(val),
            Forward(ref mut val) => Forward(val),
        }
    }

    #[inline]
    fn formatting(&self) -> (Color, &'static str) {
        match *self {
            Success(..) => (Color::Green, "Success"),
            Failure(..) => (Color::Red, "Failure"),
            Forward(..) => (Color::Yellow, "Forward"),
        }
    }
}

impl<Y, S, E: From<Y>, F> FromResidual<Result<std::convert::Infallible, Y>> for Outcome<S, E, F> {
    fn from_residual(r: Result<std::convert::Infallible, Y>) -> Self {
        match r {
            Ok(never) => match never {},
            Err(y) => Outcome::Failure(y.into()),
        }
    }
}

impl<S, X, E: From<X>, Y, F: From<Y>> FromResidual<Outcome<!, X, Y>> for Outcome<S, E, F> {
    fn from_residual(r: Outcome<!, X, Y>) -> Self {
        match r {
            Outcome::Success(never) => match never {},
            Outcome::Failure(x) => Outcome::Failure(x.into()),
            Outcome::Forward(y) => Outcome::Forward(y.into()),
        }
    }
}

impl<A, B, C> Try for Outcome<A, B, C> {
    type Output = A;

    type Residual = Outcome<!, B, C>;

    fn from_output(x: Self::Output) -> Self {
        Outcome::Success(x)
    }

    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self {
            Outcome::Success(a) => ControlFlow::Continue(a),
            Outcome::Forward(b) => ControlFlow::Break(Outcome::Forward(b)),
            Outcome::Failure(c) => ControlFlow::Break(Outcome::Failure(c))
        }
    }
}

impl<S, E, F> fmt::Debug for Outcome<S, E, F> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Outcome::{}", self.formatting().1)
    }
}

impl<S, E, F> fmt::Display for Outcome<S, E, F> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (color, string) = self.formatting();
        write!(f, "{}", Paint::default(string).fg(color))
    }
}

#[cfg(test)]
mod test {
    use super::Outcome::{self, *};

    macro_rules! fake_try {
        ($e:block) => {
            (||{
                std::ops::Try::from_output($e)
            })()
        }
    }

    #[test]
    fn outcome_try_trait() {
        let r: Outcome<u16, String, f64> = fake_try! {{ 3 }};
        assert_eq!(r, Success(3));
        let r: Outcome<u16, String, f64> = fake_try! {{ Success::<_, &'static str, f32>(3)? }};
        assert_eq!(r, Success(3));
        let r: Outcome<u16, String, f64> = fake_try! {{ Failure::<u64, _, f32>("oops")?; 7 }};
        assert_eq!(r, Failure(String::from("oops")));
        let r: Outcome<u16, String, f64> = fake_try! {{ Forward::<u64, &'static str, _>(1234.5_f32)?; 7 }};
        assert_eq!(r, Forward(1234.5));
    }

    #[test]
    fn can_use_question_mark_on_result_in_function_returning_outcome() {
        fn demo() -> Outcome<i128, String, f32> {
            Err("problem")?;
            unreachable!()
        }
        assert_eq!(demo(), Failure(String::from("problem")));
    }
}
