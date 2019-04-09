use std::str::FromStr;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6, SocketAddr};
use std::num::{
    NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128, NonZeroIsize,
    NonZeroU8, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128, NonZeroUsize,
};
use std::convert::Infallible;

use crate::http::RawStr;

/// Trait to parse a typed value from a form value.
///
/// This trait is used by Rocket's code generation in two places:
///
///   1. Fields in structs deriving [`FromForm`](crate::request::FromForm) are
///      required to implement this trait.
///   2. Types of dynamic query parameters (`?<param>`) are required to
///      implement this trait.
///
/// # `FromForm` Fields
///
/// When deriving the `FromForm` trait, Rocket uses the `FromFormValue`
/// implementation of each field's type to validate the form input. To
/// illustrate, consider the following structure:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// #[derive(FromForm)]
/// struct Person {
///     name: String,
///     age: u16
/// }
/// ```
///
/// The `FromForm` implementation generated by Rocket will call
/// `String::from_form_value` for the `name` field, and `u16::from_form_value`
/// for the `age` field. The `Person` structure can only be created from a form
/// if both calls return successfully.
///
/// # Dynamic Query Parameters
///
/// Types of dynamic query parameters are required to implement this trait. The
/// `FromFormValue` implementation is used to parse and validate each parameter
/// according to its target type:
///
/// ```rust
/// # #![feature(proc_macro_hygiene, decl_macro)]
/// # #[macro_use] extern crate rocket;
/// # type Size = String;
/// #[get("/item?<id>&<size>")]
/// fn item(id: usize, size: Size) { /* ... */ }
/// # fn main() { }
/// ```
///
/// To generate values for `id` and `size`, Rocket calls
/// `usize::from_form_value()` and `Size::from_form_value()`, respectively.
///
/// # Validation Errors
///
/// It is sometimes desired to prevent a validation error from forwarding a
/// request to another route. The `FromFormValue` implementation for `Option<T>`
/// and `Result<T, T::Error>` make this possible. Their implementations always
/// return successfully, effectively "catching" the error.
///
/// For instance, if we wanted to know if a user entered an invalid `age` in the
/// form corresponding to the `Person` structure in the first example, we could
/// use the following structure:
///
/// ```rust
/// # use rocket::http::RawStr;
/// struct Person<'r> {
///     name: String,
///     age: Result<u16, &'r RawStr>
/// }
/// ```
///
/// The `Err` value in this case is `&RawStr` since `u16::from_form_value`
/// returns a `Result<u16, &RawStr>`.
///
/// # Provided Implementations
///
/// Rocket implements `FromFormValue` for many standard library types. Their
/// behavior is documented here.
///
///   *
///       * Primitive types: **f32, f64, isize, i8, i16, i32, i64, i128,
///         usize, u8, u16, u32, u64, u128**
///       * `IpAddr` and `SocketAddr` types: **IpAddr, Ipv4Addr, Ipv6Addr,
///         SocketAddrV4, SocketAddrV6, SocketAddr**
///       * `NonZero*` types: **NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64,
///         NonZeroI128, NonZeroIsize, NonZeroU8, NonZeroU16, NonZeroU32,
///         NonZeroU64, NonZeroU128, NonZeroUsize**
///
///     A value is validated successfully if the `from_str` method for the given
///     type returns successfully. Otherwise, the raw form value is returned as
///     the `Err` value.
///
///   * **bool**
///
///     A value is validated successfully as `true` if the the form value is
///     `"true"` or `"on"`, and as a `false` value if the form value is
///     `"false"`, `"off"`, or not present. In any other case, the raw form
///     value is returned in the `Err` value.
///
///   * **[`&RawStr`](RawStr)**
///
///     _This implementation always returns successfully._
///
///     The raw, undecoded string is returned directly without modification.
///
///   * **String**
///
///     URL decodes the form value. If the decode is successful, the decoded
///     string is returned. Otherwise, an `Err` with the original form value is
///     returned.
///
///   * **Option&lt;T>** _where_ **T: FromFormValue**
///
///     _This implementation always returns successfully._
///
///     The form value is validated by `T`'s `FromFormValue` implementation. If
///     the validation succeeds, a `Some(validated_value)` is returned.
///     Otherwise, a `None` is returned.
///
///   * **Result&lt;T, T::Error>** _where_ **T: FromFormValue**
///
///     _This implementation always returns successfully._
///
///     The from value is validated by `T`'s `FromFormvalue` implementation. The
///     returned `Result` value is returned.
///
/// # Example
///
/// This trait is generally implemented to parse and validate form values. While
/// Rocket provides parsing and validation for many of the standard library
/// types such as `u16` and `String`, you can implement `FromFormValue` for a
/// custom type to get custom validation.
///
/// Imagine you'd like to verify that some user is over some age in a form. You
/// might define a new type and implement `FromFormValue` as follows:
///
/// ```rust
/// use rocket::request::FromFormValue;
/// use rocket::http::RawStr;
///
/// struct AdultAge(usize);
///
/// impl<'v> FromFormValue<'v> for AdultAge {
///     type Error = &'v RawStr;
///
///     fn from_form_value(form_value: &'v RawStr) -> Result<AdultAge, &'v RawStr> {
///         match form_value.parse::<usize>() {
///             Ok(age) if age >= 21 => Ok(AdultAge(age)),
///             _ => Err(form_value),
///         }
///     }
/// }
/// ```
///
/// The type can then be used in a `FromForm` struct as follows:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # type AdultAge = usize;
/// #[derive(FromForm)]
/// struct Person {
///     name: String,
///     age: AdultAge
/// }
/// ```
///
/// A form using the `Person` structure as its target will only parse and
/// validate if the `age` field contains a `usize` greater than `21`.
pub trait FromFormValue<'v>: Sized {
    /// The associated error which can be returned from parsing. It is a good
    /// idea to have the return type be or contain an `&'v str` so that the
    /// unparseable string can be examined after a bad parse.
    type Error;

    /// Parses an instance of `Self` from an HTTP form field value or returns an
    /// `Error` if one cannot be parsed.
    fn from_form_value(form_value: &'v RawStr) -> Result<Self, Self::Error>;

    /// Returns a default value to be used when the form field does not exist.
    /// If this returns `None`, then the field is required. Otherwise, this
    /// should return `Some(default_value)`. The default implementation simply
    /// returns `None`.
    #[inline(always)]
    fn default() -> Option<Self> {
        None
    }
}

impl<'v> FromFormValue<'v> for &'v RawStr {
    type Error = Infallible;

    // This just gives the raw string.
    #[inline(always)]
    fn from_form_value(v: &'v RawStr) -> Result<Self, Self::Error> {
        Ok(v)
    }
}

impl<'v> FromFormValue<'v> for String {
    type Error = &'v RawStr;

    // This actually parses the value according to the standard.
    #[inline(always)]
    fn from_form_value(v: &'v RawStr) -> Result<Self, Self::Error> {
        v.url_decode().map_err(|_| v)
    }
}

impl<'v> FromFormValue<'v> for bool {
    type Error = &'v RawStr;

    fn from_form_value(v: &'v RawStr) -> Result<Self, Self::Error> {
        match v.as_str() {
            "on" | "true" => Ok(true),
            "off" | "false" => Ok(false),
            _ => Err(v),
        }
    }

    #[inline(always)]
    fn default() -> Option<bool> {
        Some(false)
    }
}

macro_rules! impl_with_fromstr {
    ($($T:ident),+) => ($(
        impl<'v> FromFormValue<'v> for $T {
            type Error = &'v RawStr;

            #[inline(always)]
            fn from_form_value(v: &'v RawStr) -> Result<Self, Self::Error> {
                $T::from_str(v.as_str()).map_err(|_| v)
            }
        }
    )+)
}

impl_with_fromstr!(
    f32, f64, isize, i8, i16, i32, i64, i128, usize, u8, u16, u32, u64, u128,
    NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128, NonZeroIsize,
    NonZeroU8, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128, NonZeroUsize,
    IpAddr, Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6, SocketAddr
);

impl<'v, T: FromFormValue<'v>> FromFormValue<'v> for Option<T> {
    type Error = Infallible;

    #[inline(always)]
    fn from_form_value(v: &'v RawStr) -> Result<Self, Self::Error> {
        match T::from_form_value(v) {
            Ok(v) => Ok(Some(v)),
            Err(_) => Ok(None),
        }
    }

    #[inline(always)]
    fn default() -> Option<Option<T>> {
        Some(None)
    }
}

// // TODO: Add more useful implementations (range, regex, etc.).
impl<'v, T: FromFormValue<'v>> FromFormValue<'v> for Result<T, T::Error> {
    type Error = Infallible;

    #[inline(always)]
    fn from_form_value(v: &'v RawStr) -> Result<Self, Self::Error> {
        match T::from_form_value(v) {
            ok@Ok(_) => Ok(ok),
            e@Err(_) => Ok(e),
        }
    }
}
