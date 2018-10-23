use std::fmt;
use std::path::{Path, PathBuf};
use std::borrow::Cow;

use {RawStr, uri::Uri};

use percent_encoding::{utf8_percent_encode, DEFAULT_ENCODE_SET};

mod priv_encode_set {
    /// This encode set is used for strings where '/' characters are known to be
    /// safe; all other special path segment characters are encoded.
    define_encode_set! { pub PATH_ENCODE_SET = [super::DEFAULT_ENCODE_SET] | {'%'} }
}

use self::priv_encode_set::PATH_ENCODE_SET;

/// Trait implemented by types that can be displayed as part of a URI in `uri!`.
///
/// Types implementing this trait can be displayed in a URI-safe manner. Unlike
/// `Display`, the string written by a `UriDisplay` implementation must be
/// URI-safe. In practice, this means that the string must either be
/// percent-encoded or consist only of characters that are alphanumeric, "-",
/// ".", "_", or "~" - the "unreserved" characters.
///
/// # Code Generation
///
/// When the `uri!` macro is used to generate a URI for a route, the types for
/// the route's URI parameters must implement `UriDisplay`. The `UriDisplay`
/// implementation for these types is used when generating the URI.
///
/// To illustrate `UriDisplay`'s role in code generation for `uri!`, consider
/// the following route:
///
/// ```rust
/// # #![feature(proc_macro_hygiene, decl_macro)]
/// # #[macro_use] extern crate rocket;
/// # type T = ();
/// #[get("/item/<id>?<track>")]
/// fn get_item(id: i32, track: String) -> T { /* .. */ }
/// ```
///
/// A URI for this route can be generated as follows:
///
/// ```rust
/// # #![feature(proc_macro_hygiene, decl_macro)]
/// # #[macro_use] extern crate rocket;
/// # type T = ();
/// # #[get("/item/<id>?<track>")]
/// # fn get_item(id: i32, track: String) -> T { /* .. */ }
/// #
/// // With unnamed parameters.
/// uri!(get_item: 100, "inbound");
///
/// // With named parameters.
/// uri!(get_item: id = 100, track = "inbound");
/// uri!(get_item: track = "inbound", id = 100);
/// ```
///
/// After verifying parameters and their types, Rocket will generate code
/// similar to the following:
///
/// ```rust
/// # extern crate rocket;
/// # use rocket::http::uri::UriDisplay;
/// #
/// format!("/item/{}?track={}", &100 as &UriDisplay, &"inbound" as &UriDisplay);
/// ```
///
/// For this expression  to typecheck, both `i32` and `Value` must implement
/// `UriDisplay`. As can be seen, the implementation will be used to display the
/// value in a URI-safe manner.
///
/// [`uri!`]: /rocket_codegen/#typed-uris-uri
///
/// # Provided Implementations
///
/// Rocket implements `UriDisplay` for several built-in types. Their behavior is
/// documented here.
///
///   * **i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64, bool,
///     IpAddr, Ipv4Addr, Ipv6Addr**
///
///     The implementation of `UriDisplay` for these types is identical to the
///     `Display` implementation.
///
///   * **[`&RawStr`](RawStr), `String`, `&str`,
///     `Cow<str>`**
///
///     The string is percent encoded.
///
///   * **`&T`, `&mut T`** _where_ **`T: UriDisplay`**
///
///     Uses the implementation of `UriDisplay` for `T`.
///
/// # Implementing
///
/// Implementing `UriDisplay` is similar to implementing
/// [`Display`](::std::fmt::Display) with the caveat that extra care must be
/// taken to ensure that the written string is URI-safe. As mentioned before, in
/// practice, this means that the string must either be percent-encoded or
/// consist only of characters that are alphanumeric, "-", ".", "_", or "~".
///
/// When manually implementing `UriDisplay` for your types, you should defer to
/// existing implementations of `UriDisplay` as much as possible. In the example
/// below, for instance, `Name`'s implementation defers to `String`'s
/// implementation. To percent-encode a string, use
/// [`Uri::percent_encode()`](uri::Uri::percent_encode()).
///
/// ## Example
///
/// The following snippet consists of a `Name` type that implements both
/// `FromParam` and `UriDisplay`. The `FromParam` implementation allows `Name`
/// to be used as the target type of a dynamic parameter, while the `UriDisplay`
/// implementation allows URIs to be generated for routes with `Name` as a
/// dynamic parameter type.
///
/// ```rust
/// # #![feature(proc_macro_hygiene, decl_macro)]
/// # #[macro_use] extern crate rocket;
/// # fn main() {  }
/// use rocket::http::RawStr;
/// use rocket::request::FromParam;
///
/// struct Name(String);
///
/// impl<'r> FromParam<'r> for Name {
///     type Error = &'r RawStr;
///
///     /// Validates parameters that contain no spaces.
///     fn from_param(param: &'r RawStr) -> Result<Self, Self::Error> {
///         let decoded = param.percent_decode().map_err(|_| param)?;
///         match decoded.contains(' ') {
///             false => Ok(Name(decoded.into_owned())),
///             true => Err(param),
///         }
///     }
/// }
///
/// use std::fmt;
/// use rocket::http::uri::UriDisplay;
/// use rocket::response::Redirect;
///
/// impl UriDisplay for Name {
///     /// Delegates to the `UriDisplay` implementation for `String` to ensure
///     /// that the written string is URI-safe. In this case, the string will
///     /// be percent encoded.
///     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
///         UriDisplay::fmt(&self.0, f)
///     }
/// }
///
/// #[get("/name/<name>")]
/// fn redirector(name: Name) -> Redirect {
///     Redirect::to(uri!(real: name))
/// }
///
/// #[get("/<name>")]
/// fn real(name: Name) -> String {
///     format!("Hello, {}!", name.0)
/// }
/// ```
pub trait UriDisplay {
    /// Formats `self` in a URI-safe manner using the given formatter.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result;
}

impl<'a> fmt::Display for &'a UriDisplay {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        UriDisplay::fmt(*self, f)
    }
}

/// Percent-encodes the raw string.
impl UriDisplay for RawStr {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", Uri::percent_encode((*self).as_str()))
    }
}

/// Percent-encodes the raw string.
impl UriDisplay for str {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", Uri::percent_encode(self))
    }
}

/// Percent-encodes the raw string.
impl<'a> UriDisplay for Cow<'a, str> {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", Uri::percent_encode(self))
    }
}

/// Percent-encodes the raw string.
impl UriDisplay for String {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", Uri::percent_encode(self.as_str()))
    }
}

/// Percent-encodes each segment in the path.
impl UriDisplay for PathBuf {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let string = self.to_string_lossy();
        let enc: Cow<str> = utf8_percent_encode(&string, PATH_ENCODE_SET).into();
        write!(f, "{}", enc)
    }
}

/// Percent-encodes each segment in the path.
impl UriDisplay for Path {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let string = self.to_string_lossy();
        let enc: Cow<str> = utf8_percent_encode(&string, PATH_ENCODE_SET).into();
        write!(f, "{}", enc)
    }
}

macro_rules! impl_with_display {
    ($($T:ty),+) => {$(
        /// This implementation is identical to the `Display` implementation.
        impl UriDisplay for $T  {
            #[inline(always)]
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                fmt::Display::fmt(self, f)
            }
        }
    )+}
}

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

impl_with_display! {
    i8, i16, i32, i64, i128, isize,
    u8, u16, u32, u64, u128, usize,
    f32, f64, bool,
    IpAddr, Ipv4Addr, Ipv6Addr
}

macro_rules! impl_for_ref {
    ($($T:ty),+) => {$(
        /// Uses the implementation of `UriDisplay` for `T`.
        impl<'a, T: UriDisplay + ?Sized> UriDisplay for $T {
            #[inline(always)]
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                UriDisplay::fmt(*self, f)
            }
        }
    )+}
}

impl_for_ref!(&'a mut T, &'a T);
