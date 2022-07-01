use std::str::FromStr;
use std::path::PathBuf;

use crate::http::uri::{Segments, error::PathError, fmt::Path};

/// Trait to convert a dynamic path segment string to a concrete value.
///
/// This trait is used by Rocket's code generation facilities to parse dynamic
/// path segment string values into a given type. That is, when a path contains
/// a dynamic segment `<param>` where `param` has some type `T` that implements
/// `FromParam`, `T::from_param` will be called.
///
/// # Forwarding
///
/// If the conversion fails, the incoming request will be forwarded to the next
/// matching route, if any. For instance, consider the following route and
/// handler for the dynamic `"/<id>"` path:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// #[get("/<id>")]
/// fn hello(id: usize) -> String {
/// # let _id = id;
/// # /*
///     ...
/// # */
/// # "".to_string()
/// }
/// # fn main() {  }
/// ```
///
/// If `usize::from_param` returns an `Ok(usize)` variant, the encapsulated
/// value is used as the `id` function parameter. If not, the request is
/// forwarded to the next matching route. Since there are no additional matching
/// routes, this example will result in a 404 error for requests with invalid
/// `id` values.
///
/// # Catching Errors
///
/// Sometimes, a forward is not desired, and instead, we simply want to know
/// that the dynamic path segment could not be parsed into some desired type
/// `T`. In these cases, types of `Option<T>` or `Result<T, T::Error>` can be
/// used. These types implement `FromParam` themselves. Their implementations
/// always return successfully, so they never forward. They can be used to
/// determine if the `FromParam` call failed and to retrieve the error value
/// from the failed `from_param` call.
///
/// For instance, imagine you've asked for an `<id>` as a `usize`. To determine
/// when the `<id>` was not a valid `usize` and retrieve the string that failed
/// to parse, you can use a `Result<usize, &str>` type for the `<id>` parameter
/// as follows:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// #[get("/<id>")]
/// fn hello(id: Result<usize, &str>) -> String {
///     match id {
///         Ok(id_num) => format!("usize: {}", id_num),
///         Err(string) => format!("Not a usize: {}", string)
///     }
/// }
/// # fn main() {  }
/// ```
///
/// # Provided Implementations
///
/// Rocket implements `FromParam` for several standard library types. Their
/// behavior is documented here.
///
///   *
///       * Primitive types: **f32, f64, isize, i8, i16, i32, i64, i128,
///         usize, u8, u16, u32, u64, u128, bool**
///       * `IpAddr` and `SocketAddr` types: **IpAddr, Ipv4Addr, Ipv6Addr,
///         SocketAddrV4, SocketAddrV6, SocketAddr**
///       * `NonZero*` types: **NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64,
///         NonZeroI128, NonZeroIsize, NonZeroU8, NonZeroU16, NonZeroU32,
///         NonZeroU64, NonZeroU128, NonZeroUsize**
///
///     A value is parsed successfully if the `from_str` method from the given
///     type returns successfully. Otherwise, the raw path segment is returned
///     in the `Err` value.
///
///   * **&str, String**
///
///     _This implementation always returns successfully._
///
///     Returns the percent-decoded path segment with invalid UTF-8 byte
///     sequences replaced by � U+FFFD.
///
///   * **Option&lt;T>** _where_ **T: FromParam**
///
///     _This implementation always returns successfully._
///
///     The path segment is parsed by `T`'s `FromParam` implementation. If the
///     parse succeeds, a `Some(parsed_value)` is returned. Otherwise, a `None`
///     is returned.
///
///   * **Result&lt;T, T::Error>** _where_ **T: FromParam**
///
///     _This implementation always returns successfully._
///
///     The path segment is parsed by `T`'s `FromParam` implementation. The
///     returned `Result` value is returned.
///
/// # Example
///
/// Say you want to parse a segment of the form:
///
/// ```text
/// [a-zA-Z]+:[0-9]+
/// ```
///
/// into the following structure, where the string before the `:` is stored in
/// `key` and the number after the colon is stored in `value`:
///
/// ```rust
/// struct MyParam<'r> {
///     key: &'r str,
///     value: usize
/// }
/// ```
///
/// The following implementation accomplishes this:
///
/// ```rust
/// use rocket::request::FromParam;
/// # #[allow(dead_code)]
/// # struct MyParam<'r> { key: &'r str, value: usize }
///
/// impl<'r> FromParam<'r> for MyParam<'r> {
///     type Error = &'r str;
///
///     fn from_param(param: &'r str) -> Result<Self, Self::Error> {
///         // We can convert `param` into a `str` since we'll check every
///         // character for safety later.
///         let (key, val_str) = match param.find(':') {
///             Some(i) if i > 0 => (&param[..i], &param[(i + 1)..]),
///             _ => return Err(param)
///         };
///
///         if !key.chars().all(|c| c.is_ascii_alphabetic()) {
///             return Err(param);
///         }
///
///         val_str.parse()
///             .map(|value| MyParam { key, value })
///             .map_err(|_| param)
///     }
/// }
/// ```
///
/// With the implementation, the `MyParam` type can be used as the target of a
/// dynamic path segment:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # use rocket::request::FromParam;
/// # #[allow(dead_code)]
/// # struct MyParam<'r> { key: &'r str, value: usize }
/// # impl<'r> FromParam<'r> for MyParam<'r> {
/// #     type Error = &'r str;
/// #     fn from_param(param: &'r str) -> Result<Self, Self::Error> {
/// #         Err(param)
/// #     }
/// # }
/// #
/// #[get("/<key_val>")]
/// fn hello(key_val: MyParam) -> String {
/// # let _kv = key_val;
/// # /*
///     ...
/// # */
/// # "".to_string()
/// }
/// # fn main() {  }
/// ```
pub trait FromParam<'a>: Sized {
    /// The associated error to be returned if parsing/validation fails.
    type Error: std::fmt::Debug;

    /// Parses and validates an instance of `Self` from a path parameter string
    /// or returns an `Error` if parsing or validation fails.
    fn from_param(param: &'a str) -> Result<Self, Self::Error>;
}

impl<'a> FromParam<'a> for &'a str {
    type Error = std::convert::Infallible;

    #[inline(always)]
    fn from_param(param: &'a str) -> Result<&'a str, Self::Error> {
        Ok(param)
    }
}

impl<'a> FromParam<'a> for String {
    type Error = std::convert::Infallible;

    #[inline(always)]
    fn from_param(param: &'a str) -> Result<String, Self::Error> {
        // TODO: Tell the user they're being inefficient?
        Ok(param.to_string())
    }
}

macro_rules! impl_with_fromstr {
    ($($T:ty),+) => ($(
        impl<'a> FromParam<'a> for $T {
            type Error = &'a str;

            #[inline(always)]
            fn from_param(param: &'a str) -> Result<Self, Self::Error> {
                <$T as FromStr>::from_str(param).map_err(|_| param)
            }
        }
    )+)
}

use std::num::{
    NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128, NonZeroIsize,
    NonZeroU8, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128, NonZeroUsize,
};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6, SocketAddr};

impl_with_fromstr! {
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64,
    NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128, NonZeroIsize,
    NonZeroU8, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128, NonZeroUsize,
    bool, IpAddr, Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6, SocketAddr
}

impl<'a> FromParam<'a> for PathBuf {
    type Error = PathError;

    #[inline]
    fn from_param(param: &'a str) -> Result<Self, Self::Error> {
        use crate::http::private::Indexed;

        let segments = &[Indexed::Indexed(0, param.len())];
        Segments::new(param.into(), segments).to_path_buf(false)
    }
}

impl<'a, T: FromParam<'a>> FromParam<'a> for Result<T, T::Error> {
    type Error = std::convert::Infallible;

    #[inline]
    fn from_param(param: &'a str) -> Result<Self, Self::Error> {
        match T::from_param(param) {
            Ok(val) => Ok(Ok(val)),
            Err(e) => Ok(Err(e)),
        }
    }
}

impl<'a, T: FromParam<'a>> FromParam<'a> for Option<T> {
    type Error = std::convert::Infallible;

    #[inline]
    fn from_param(param: &'a str) -> Result<Self, Self::Error> {
        match T::from_param(param) {
            Ok(val) => Ok(Some(val)),
            Err(_) => Ok(None)
        }
    }
}

/// Trait to convert _many_ dynamic path segment strings to a concrete value.
///
/// This is the `..` analog to [`FromParam`], and its functionality is identical
/// to it with one exception: this trait applies to segment parameters of the
/// form `<param..>`, where `param` is of some type `T` that implements
/// `FromSegments`. `T::from_segments` is called to convert the matched segments
/// (via the [`Segments`] iterator) into the implementing type.
///
/// # Provided Implementations
///
/// **`PathBuf`**
///
/// The `PathBuf` implementation constructs a path from the segments iterator.
/// Each segment is percent-decoded. If a segment equals ".." before or after
/// decoding, the previous segment (if any) is omitted. For security purposes,
/// any other segments that begin with "*" or "." are ignored.  If a
/// percent-decoded segment results in invalid UTF8, an `Err` is returned with
/// the `Utf8Error`.
pub trait FromSegments<'r>: Sized {
    /// The associated error to be returned when parsing fails.
    type Error: std::fmt::Debug;

    /// Parses an instance of `Self` from many dynamic path parameter strings or
    /// returns an `Error` if one cannot be parsed.
    fn from_segments(segments: Segments<'r, Path>) -> Result<Self, Self::Error>;
}

impl<'r> FromSegments<'r> for Segments<'r, Path> {
    type Error = std::convert::Infallible;

    #[inline(always)]
    fn from_segments(segments: Self) -> Result<Self, Self::Error> {
        Ok(segments)
    }
}

/// Creates a `PathBuf` from a `Segments` iterator. The returned `PathBuf` is
/// percent-decoded. If a segment is equal to "..", the previous segment (if
/// any) is skipped.
///
/// For security purposes, if a segment meets any of the following conditions,
/// an `Err` is returned indicating the condition met:
///
///   * Decoded segment starts with any of: `.` (except `..`), `*`
///   * Decoded segment ends with any of: `:`, `>`, `<`
///   * Decoded segment contains any of: `/`
///   * On Windows, decoded segment contains any of: `\`
///   * Percent-encoding results in invalid UTF8.
///
/// As a result of these conditions, a `PathBuf` derived via `FromSegments` is
/// safe to interpolate within, or use as a suffix of, a path without additional
/// checks.
impl FromSegments<'_> for PathBuf {
    type Error = PathError;

    fn from_segments(segments: Segments<'_, Path>) -> Result<Self, Self::Error> {
        segments.to_path_buf(false)
    }
}

impl<'r, T: FromSegments<'r>> FromSegments<'r> for Result<T, T::Error> {
    type Error = std::convert::Infallible;

    #[inline]
    fn from_segments(segments: Segments<'r, Path>) -> Result<Result<T, T::Error>, Self::Error> {
        match T::from_segments(segments) {
            Ok(val) => Ok(Ok(val)),
            Err(e) => Ok(Err(e)),
        }
    }
}

impl<'r, T: FromSegments<'r>> FromSegments<'r> for Option<T> {
    type Error = std::convert::Infallible;

    #[inline]
    fn from_segments(segments: Segments<'r, Path>) -> Result<Option<T>, Self::Error> {
        match T::from_segments(segments) {
            Ok(val) => Ok(Some(val)),
            Err(_) => Ok(None)
        }
    }
}
