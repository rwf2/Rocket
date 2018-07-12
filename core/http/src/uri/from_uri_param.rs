use std::path::{Path, PathBuf};

use {RawStr, uri::UriDisplay};

/// Conversion trait for parameters used in `uri!` invocations.
///
/// This trait is invoked once per expression passed into a [`uri!`] invocation.
/// In particular, for a route URI parameter of type `T` and a user-supplied
/// expression of type `S`, `<T as FromUriParam<S>>::from_uri_param` is
/// invoked. The returned value is used in place of the user's value and
/// rendered using its [`UriDisplay`] implementation.
///
/// This trait allows types that differ from the route URI parameter's types to
/// be used in their place at no cost. For instance, the following
/// implementation, provided by Rocket, allows an `&str` to be used in a `uri!`
/// invocation for route URI parameters declared as `String`:
///
/// ```rust,ignore
/// impl<'a> FromUriParam<&'a str> for String { type Target = &'a str; }
/// ```
///
/// Because the `Target` type is the same as the input type, the conversion is a
/// no-op and free of cost, allowing an `&str` to be used in place of a
/// `String` without penalty. A similar no-op conversion exists for [`&RawStr`]:
///
/// ```rust,ignore
/// impl<'a, 'b> FromUriParam<&'a str> for &'b RawStr { type Target = &'a str; }
/// ```
///
/// [`&RawStr`]: /rocket/http/struct.RawStr.html
///
/// # Implementing
///
/// Rocket provides a blanket implementation for all types that implement
/// [`UriDisplay`]. As such, this trait typically does not need to be implemented.
/// Instead, implement [`UriDisplay`].
///
/// This trait should only be implemented when you'd like to allow a type
/// different from the route's declared type to be used in its place in a `uri!`
/// invocation. For instance, if the route has a type of `T` and you'd like to
/// use a type of `S` in a `uri!` invocation, you'd implement `FromUriParam<T>
/// for S`.
///
/// This is typically only warranted for owned-value types with
/// corresponding reference types: `String` and `&str`, for instance. In this
/// case, it's desirable to allow an `&str` to be used in place of a `String`.
///
/// When implementing `FromUriParam`, be aware that Rocket will use the
/// [`UriDisplay`] implementation of `Target`, _not_ of the source type.
/// Incorrect implementations can result in creating unsafe URIs.
///
/// # Example
///
/// The following example implements `FromUriParam<(&str, &str)>` for a `User`
/// type. The implementation allows an `(&str, &str)` type to be used in a
/// `uri!` invocation where a `User` type is expected.
///
/// ```rust
/// # extern crate rocket;
/// use std::fmt;
///
/// use rocket::http::RawStr;
/// use rocket::http::uri::{UriDisplay, FromUriParam};
///
/// # /*
/// #[derive(FromForm)]
/// # */
/// struct User<'a> {
///     name: &'a RawStr,
///     nickname: String,
/// }
///
/// impl<'a> UriDisplay for User<'a> {
///     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
///         write!(f, "name={}&nickname={}",
///                &self.name.replace(' ', "+") as &UriDisplay,
///                &self.nickname.replace(' ', "+") as &UriDisplay)
///     }
/// }
///
/// impl<'a, 'b> FromUriParam<(&'a str, &'b str)> for User<'a> {
///     type Target = User<'a>;
///     fn from_uri_param((name, nickname): (&'a str, &'b str)) -> User<'a> {
///         User { name: name.into(), nickname: nickname.to_string() }
///     }
/// }
/// ```
///
/// With these implementations, the following typechecks:
///
/// ```rust,ignore
/// #[post("/<name>?<query>")]
/// fn some_route(name: &RawStr, query: User) -> T { .. }
///
/// uri!(some_route: name = "hey", query = ("Robert Mike", "Bob"));
/// // => "/hey?name=Robert+Mike&nickname=Bob"
/// ```
///
/// [`uri!`]: /rocket_codegen/#typed-uris-uri
/// [`UriDisplay`]: /rocket/http/uri/trait.UriDisplay.html
pub trait FromUriParam<T>: UriDisplay {
    /// The resulting type of this conversion.
    type Target: UriDisplay;

    /// Converts a value of type `T` into a value of type `Self::Target`. The
    /// resulting value of type `Self::Target` will be rendered into a URI using
    /// its [`UriDisplay`](/rocket/http/uri/trait.UriDisplay.html)
    /// implementation.
    fn from_uri_param(param: T) -> Self::Target;
}

impl<T: UriDisplay> FromUriParam<T> for T {
    type Target = T;
    #[inline(always)]
    fn from_uri_param(param: T) -> T { param }
}

impl<'a, T: UriDisplay> FromUriParam<&'a T> for T {
    type Target = &'a T;
    #[inline(always)]
    fn from_uri_param(param: &'a T) -> &'a T { param }
}

impl<'a, T: UriDisplay> FromUriParam<&'a mut T> for T {
    type Target = &'a mut T;
    #[inline(always)]
    fn from_uri_param(param: &'a mut T) -> &'a mut T { param }
}

/// A no cost conversion allowing an `&str` to be used in place of a `String`.
impl<'a> FromUriParam<&'a str> for String {
    type Target = &'a str;
    #[inline(always)]
    fn from_uri_param(param: &'a str) -> &'a str { param }
}

/// A no cost conversion allowing an `&str` to be used in place of an `&RawStr`.
impl<'a, 'b> FromUriParam<&'a str> for &'b RawStr {
    type Target = &'a str;
    #[inline(always)]
    fn from_uri_param(param: &'a str) -> &'a str { param }
}

/// A no cost conversion allowing an `&Path` to be used in place of a `PathBuf`.
impl<'a> FromUriParam<&'a Path> for PathBuf {
    type Target = &'a Path;
    #[inline(always)]
    fn from_uri_param(param: &'a Path) -> &'a Path { param }
}

/// A no cost conversion allowing an `&str` to be used in place of a `PathBuf`.
impl<'a> FromUriParam<&'a str> for PathBuf {
    type Target = &'a Path;
    #[inline(always)]
    fn from_uri_param(param: &'a str) -> &'a Path {
        Path::new(param)
    }
}
