use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6, SocketAddr};
use std::str::FromStr;

use error::Error;
use http::uri::URI;

/// Trait to create instance of some type from a form value; expected from field
/// types in structs deriving `FromForm`.
///
/// # Examples
///
/// This trait is generally implemented when verifying form inputs. For example,
/// if you'd like to verify that some user is over some age in a form, then you
/// might define a new type and implement `FromFormValue` as follows:
///
/// ```rust
/// use rocket::request::FromFormValue;
/// use rocket::Error;
///
/// struct AdultAge(usize);
///
/// impl<'v> FromFormValue<'v> for AdultAge {
///     type Error = &'v str;
///
///     fn from_form_value(form_value: &'v str) -> Result<AdultAge, &'v str> {
///         match usize::from_form_value(form_value) {
///             Ok(age) if age >= 21 => Ok(AdultAge(age)),
///             _ => Err(form_value),
///         }
///     }
/// }
/// ```
///
/// This type can then be used in a `FromForm` struct as follows:
///
/// ```rust,ignore
/// #[derive(FromForm)]
/// struct User {
///     age: AdultAge,
///     ...
/// }
/// ```
pub trait FromFormValue<'v>: Sized {
    /// The associated error which can be returned from parsing. It is a good
    /// idea to have the return type be or contain an `&'v str` so that the
    /// unparseable string can be examined after a bad parse.
    type Error;

    /// Parses an instance of `Self` from an HTTP form field value or returns an
    /// `Error` if one cannot be parsed.
    fn from_form_value(form_value: &'v str) -> Result<Self, Self::Error>;

    /// Returns a default value to be used when the form field does not exist.
    /// If this returns `None`, then the field is required. Otherwise, this
    /// should return `Some(default_value)`. The default implementation simply
    /// returns `None`.
    fn default() -> Option<Self> {
        None
    }
}

impl<'v> FromFormValue<'v> for &'v str {
    type Error = Error;

    // This just gives the raw string.
    fn from_form_value(v: &'v str) -> Result<Self, Self::Error> {
        Ok(v)
    }
}

impl<'v> FromFormValue<'v> for String {
    type Error = &'v str;

    // This actually parses the value according to the standard.
    fn from_form_value(v: &'v str) -> Result<Self, Self::Error> {
        let replaced = v.replace("+", " ");
        match URI::percent_decode(replaced.as_bytes()) {
            Err(_) => Err(v),
            Ok(string) => Ok(string.into_owned())
        }
    }
}

impl<'v> FromFormValue<'v> for bool {
    type Error = &'v str;

    fn from_form_value(v: &'v str) -> Result<Self, Self::Error> {
        match v {
            "on" | "true" => Ok(true),
            "off" | "false" => Ok(false),
            _ => Err(v),
        }
    }
}

macro_rules! impl_with_fromstr {
    ($($T:ident),+) => ($(
        impl<'v> FromFormValue<'v> for $T {
            type Error = &'v str;
            fn from_form_value(v: &'v str) -> Result<Self, Self::Error> {
                $T::from_str(v).map_err(|_| v)
            }
        }
    )+)
}

impl_with_fromstr!(f32, f64, isize, i8, i16, i32, i64, usize, u8, u16, u32, u64,
    IpAddr, Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6, SocketAddr);

impl<'v, T: FromFormValue<'v>> FromFormValue<'v> for Option<T> {
    type Error = Error;

    fn from_form_value(v: &'v str) -> Result<Self, Self::Error> {
        match T::from_form_value(v) {
            Ok(v) => Ok(Some(v)),
            Err(_) => Ok(None),
        }
    }

    fn default() -> Option<Option<T>> {
        Some(None)
    }
}

// TODO: Add more useful implementations (range, regex, etc.).
impl<'v, T: FromFormValue<'v>> FromFormValue<'v> for Result<T, T::Error> {
    type Error = Error;

    fn from_form_value(v: &'v str) -> Result<Self, Self::Error> {
        match T::from_form_value(v) {
            ok@Ok(_) => Ok(ok),
            e@Err(_) => Ok(e),
        }
    }
}

