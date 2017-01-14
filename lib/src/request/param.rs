use std::str::FromStr;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6, SocketAddr};
use std::path::PathBuf;
use std::fmt::Debug;

use http::uri::{URI, Segments, SegmentError};

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
/// # #![feature(plugin)]
/// # #![plugin(rocket_codegen)]
/// # extern crate rocket;
/// #[get("/<id>")]
/// fn hello(id: usize) -> String {
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
/// used. These types implement `FromParam` themeselves. Their implementations
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
/// # #![feature(plugin)]
/// # #![plugin(rocket_codegen)]
/// # extern crate rocket;
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
///   * **f32, f64, isize, i8, i16, i32, i64, usize, u8, u16, u32, u64, bool**
///
///   **IpAddr, Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6, SocketAddr**
///
///     A value is parse successfully if the `from_str` method from the given
///     type returns successfully. Otherwise, the raw path segment is returned
///     in the `Err` value.
///
///   * **str**
///
///     _This implementation always returns successfully._
///
///     The path segment is passed directly with no modification.
///
///   * **String**
///
///     Percent decodes the path segment. If the decode is successful, the
///     decoded string is returned. Otherwise, an `Err` with the original path
///     segment is returned.
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
/// # `str` vs. `String`
///
/// Paths are URL encoded. As a result, the `str` `FromParam` implementation
/// returns the raw, URL encoded version of the path segment string. On the
/// other hand, `String` decodes the path parameter, but requires an allocation
/// to do so. This tradeoff is similiar to that of form values, and you should
/// use whichever makes sense for your application.
///
/// # Example
///
/// Say you want to parse a segment of the form:
///
/// ```ignore
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
/// # struct MyParam<'r> { key: &'r str, value: usize }
///
/// impl<'r> FromParam<'r> for MyParam<'r> {
///     type Error = &'r str;
///
///     fn from_param(param: &'r str) -> Result<MyParam<'r>, &'r str> {
///         let (key, val_str) = match param.find(':') {
///             Some(i) if i > 0 => (&param[..i], &param[(i + 1)..]),
///             _ => return Err(param)
///         };
///
///         if !key.chars().all(|c| (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z')) {
///             return Err(param);
///         }
///
///         val_str.parse().map(|value| {
///             MyParam {
///                 key: key,
///                 value: value
///             }
///         }).map_err(|_| param)
///     }
/// }
/// ```
///
/// With the implementation, the `MyParam` type can be used as the target of a
/// dynamic path segment:
///
/// ```rust
/// # #![feature(plugin)]
/// # #![plugin(rocket_codegen)]
/// # extern crate rocket;
/// # use rocket::request::FromParam;
/// # struct MyParam<'r> { key: &'r str, value: usize }
/// # impl<'r> FromParam<'r> for MyParam<'r> {
/// #     type Error = &'r str;
/// #     fn from_param(param: &'r str) -> Result<MyParam<'r>, &'r str> {
/// #         Err(param)
/// #     }
/// # }
/// #
/// #[get("/<key_val>")]
/// fn hello(key_val: MyParam) -> String {
/// # /*
///     ...
/// # */
/// # "".to_string()
/// }
/// # fn main() {  }
/// ```
pub trait FromParam<'a>: Sized {
    /// The associated error to be returned when parsing fails.
    type Error: Debug;

    /// Parses an instance of `Self` from a dynamic path parameter string or
    /// returns an `Error` if one cannot be parsed.
    fn from_param(param: &'a str) -> Result<Self, Self::Error>;
}

impl<'a> FromParam<'a> for &'a str {
    type Error = ();
    fn from_param(param: &'a str) -> Result<&'a str, Self::Error> {
        Ok(param)
    }
}

impl<'a> FromParam<'a> for String {
    type Error = &'a str;
    fn from_param(p: &'a str) -> Result<String, Self::Error> {
        URI::percent_decode(p.as_bytes()).map_err(|_| p).map(|s| s.into_owned())
    }
}

macro_rules! impl_with_fromstr {
    ($($T:ident),+) => ($(
        impl<'a> FromParam<'a> for $T {
            type Error = &'a str;
            fn from_param(param: &'a str) -> Result<Self, Self::Error> {
                $T::from_str(param).map_err(|_| param)
            }
        }
    )+)
}

impl_with_fromstr!(f32, f64, isize, i8, i16, i32, i64, usize, u8, u16, u32, u64,
       bool, IpAddr, Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6,
       SocketAddr);

impl<'a, T: FromParam<'a>> FromParam<'a> for Result<T, T::Error> {
    type Error = ();
    fn from_param(p: &'a str) -> Result<Self, Self::Error> {
        Ok(match T::from_param(p) {
            Ok(val) => Ok(val),
            Err(e) => Err(e),
        })
    }
}

impl<'a, T: FromParam<'a>> FromParam<'a> for Option<T> {
    type Error = ();
    fn from_param(p: &'a str) -> Result<Self, Self::Error> {
        Ok(match T::from_param(p) {
            Ok(val) => Some(val),
            Err(_) => None
        })
    }
}

/// Trait to convert _many_ dynamic path segment strings to a concrete value.
///
/// This is the `..` analog to [FromParam](trait.FromParam.html), and its
/// functionality is identical to it with one exception: this trait applies to
/// segment parameters of the form `<param..>`, where `param` is of some type
/// `T` that implements `FromSegments`. `T::from_segments` is called to convert
/// the matched segments (via the
/// [Segments](/rocket/http/uri/struct.Segments.html) iterator) into the
/// implementing type.
///
/// # Provided Implementations
///
/// Rocket implements `FromParam` for `PathBuf`. The `PathBuf` implementation
/// constructs a path from the segments iterator. Each segment is
/// percent-decoded. If a segment equals ".." before or after decoding, the
/// previous segment (if any) is omitted. For security purposes, any other
/// segments that begin with "*" or "." are ignored.  If a percent-decoded
/// segment results in invalid UTF8, an `Err` is returned with the `Utf8Error`.
pub trait FromSegments<'a>: Sized {
    /// The associated error to be returned when parsing fails.
    type Error: Debug;

    /// Parses an instance of `Self` from many dynamic path parameter strings or
    /// returns an `Error` if one cannot be parsed.
    fn from_segments(segments: Segments<'a>) -> Result<Self, Self::Error>;
}

impl<'a> FromSegments<'a> for Segments<'a> {
    type Error = ();

    fn from_segments(segments: Segments<'a>) -> Result<Segments<'a>, ()> {
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
///   * Decoded segment starts with any of: `.`, `*`
///   * Decoded segment ends with any of: `:`, `>`, `<`
///   * Decoded segment contains any of: `/`
///   * On Windows, decoded segment contains any of: '\'
///   * Percent-encoding results in invalid UTF8.
///
/// As a result of these conditions, a `PathBuf` derived via `FromSegments` is
/// safe to interpolate within, or use as a suffix of, a path without additional
/// checks.
impl<'a> FromSegments<'a> for PathBuf {
    type Error = SegmentError;

    fn from_segments(segments: Segments<'a>) -> Result<PathBuf, SegmentError> {
        let mut buf = PathBuf::new();
        for segment in segments {
            let decoded = URI::percent_decode(segment.as_bytes())
                .map_err(|e| SegmentError::Utf8(e))?;

            if decoded == ".." {
                buf.pop();
            } else if decoded.starts_with('.') {
                return Err(SegmentError::BadStart('.'))
            } else if decoded.starts_with('*') {
                return Err(SegmentError::BadStart('*'))
            } else if decoded.ends_with(':') {
                return Err(SegmentError::BadEnd(':'))
            } else if decoded.ends_with('>') {
                return Err(SegmentError::BadEnd('>'))
            } else if decoded.ends_with('<') {
                return Err(SegmentError::BadEnd('<'))
            } else if decoded.contains('/') {
                return Err(SegmentError::BadChar('/'))
            } else if cfg!(windows) && decoded.contains('\\') {
                return Err(SegmentError::BadChar('\\'))
            } else {
                buf.push(&*decoded)
            }
        }

        Ok(buf)
    }
}

impl<'a, T: FromSegments<'a>> FromSegments<'a> for Result<T, T::Error> {
    type Error = ();
    fn from_segments(segments: Segments<'a>) -> Result<Result<T, T::Error>, ()> {
        Ok(match T::from_segments(segments) {
            Ok(val) => Ok(val),
            Err(e) => Err(e),
        })
    }
}

impl<'a, T: FromSegments<'a>> FromSegments<'a> for Option<T> {
    type Error = ();
    fn from_segments(segments: Segments<'a>) -> Result<Option<T>, ()> {
        Ok(match T::from_segments(segments) {
            Ok(val) => Some(val),
            Err(_) => None
        })
    }
}
