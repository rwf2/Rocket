use std::collections::{BTreeMap, HashMap};
use std::{fmt, path};
use std::borrow::Cow;

use time::{macros::format_description, format_description::FormatItem};

use crate::RawStr;
use crate::uri::fmt::{Part, Path, Query, Formatter};

/// Trait implemented by types that can be displayed as part of a URI in
/// [`uri!`].
///
/// Types implementing this trait can be displayed in a URI-safe manner. Unlike
/// `Display`, the string written by a `UriDisplay` implementation must be
/// URI-safe. In practice, this means that the string must either be
/// percent-encoded or consist only of characters that are alphanumeric, "-",
/// ".", "_", or "~" - the "unreserved" characters.
///
/// # Marker Generic: `Path`, `Query`
///
/// The [`Part`] parameter `P` in `UriDisplay<P>` must be either [`Path`] or
/// [`Query`] (see the [`Part`] documentation for how this is enforced),
/// resulting in either `UriDisplay<Path>` or `UriDisplay<Query>`.
///
/// As the names might imply, the `Path` version of the trait is used when
/// displaying parameters in the path part of the URI while the `Query` version
/// is used when displaying parameters in the query part of the URI. These
/// distinct versions of the trait exist exactly to differentiate, at the
/// type-level, where in the URI a value is to be written to, allowing for type
/// safety in the face of differences between the two locations. For example,
/// while it is valid to use a value of `None` in the query part, omitting the
/// parameter entirely, doing so is _not_ valid in the path part. By
/// differentiating in the type system, both of these conditions can be enforced
/// appropriately through distinct implementations of `UriDisplay<Path>` and
/// `UriDisplay<Query>`.
///
/// Occasionally, the implementation of `UriDisplay` is independent of where the
/// parameter is to be displayed. When this is the case, the parameter may be
/// kept generic. That is, implementations can take the form:
///
/// ```rust
/// # extern crate rocket;
/// # use std::fmt;
/// # use rocket::http::uri::fmt::{Part, UriDisplay, Formatter};
/// # struct SomeType;
/// impl<P: Part> UriDisplay<P> for SomeType
/// # { fn fmt(&self, f: &mut Formatter<P>) -> fmt::Result { Ok(()) } }
/// ```
///
/// # Code Generation
///
/// When the [`uri!`] macro is used to generate a URI for a route, the types for
/// the route's _path_ URI parameters must implement `UriDisplay<Path>`, while
/// types in the route's query parameters must implement `UriDisplay<Query>`.
/// Any parameters ignored with `_` must be of a type that implements
/// [`Ignorable`]. The `UriDisplay` implementation for these types is used when
/// generating the URI.
///
/// To illustrate `UriDisplay`'s role in code generation for `uri!`, consider
/// the following route:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// #[get("/item/<id>?<track>")]
/// fn get_item(id: i32, track: Option<String>) { /* .. */ }
/// ```
///
/// A URI for this route can be generated as follows:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # type T = ();
/// # #[get("/item/<id>?<track>")]
/// # fn get_item(id: i32, track: Option<String>) { /* .. */ }
/// #
/// // With unnamed parameters.
/// uri!(get_item(100, Some("inbound")));
///
/// // With named parameters.
/// uri!(get_item(id = 100, track = Some("inbound")));
/// uri!(get_item(track = Some("inbound"), id = 100));
///
/// // Ignoring `track`.
/// uri!(get_item(100, _));
/// uri!(get_item(100, None as Option<String>));
/// uri!(get_item(id = 100, track = _));
/// uri!(get_item(track = _, id = 100));
/// uri!(get_item(id = 100, track = None as Option<&str>));
/// ```
///
/// After verifying parameters and their types, Rocket will generate code
/// similar (in spirit) to the following:
///
/// ```rust
/// # extern crate rocket;
/// # use rocket::http::uri::Origin;
/// # use rocket::http::uri::fmt::{UriDisplay, Path, Query};
/// #
/// Origin::parse(&format!("/item/{}?track={}",
///     &100 as &dyn UriDisplay<Path>, &"inbound" as &dyn UriDisplay<Query>));
/// ```
///
/// For this expression to typecheck, `i32` must implement `UriDisplay<Path>`
/// and `&str` must implement `UriDisplay<Query>`. What's more, when `track` is
/// ignored, `Option<String>` is required to implement [`Ignorable`]. As can be
/// seen, the implementations will be used to display the value in a URI-safe
/// manner.
///
/// [`uri!`]: rocket::uri
///
/// # Provided Implementations
///
/// Rocket implements `UriDisplay<P>` for all `P: Part` for several built-in
/// types.
///
///   * **i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32,
///     f64, bool, IpAddr, Ipv4Addr, Ipv6Addr**
///
///     The implementation of `UriDisplay` for these types is identical to the
///     `Display` implementation.
///
///   * **`String`, `&str`, `Cow<str>`**
///
///     The string is percent encoded.
///
///   * **`&T`, `&mut T`** _where_ **`T: UriDisplay`**
///
///     Uses the implementation of `UriDisplay` for `T`.
///
/// Rocket implements `UriDisplay<Path>` (but not `UriDisplay<Query>`) for
/// several built-in types.
///
///   * `T` for **`Option<T>`** _where_ **`T: UriDisplay<Path>`**
///
///     Uses the implementation of `UriDisplay` for `T::Target`.
///
///     When a type of `Option<T>` appears in a route path, use a type of `T` as
///     the parameter in `uri!`. Note that `Option<T>` itself _does not_
///     implement `UriDisplay<Path>`.
///
///   * `T` for **`Result<T, E>`** _where_ **`T: UriDisplay<Path>`**
///
///     Uses the implementation of `UriDisplay` for `T::Target`.
///
///     When a type of `Result<T, E>` appears in a route path, use a type of `T`
///     as the parameter in `uri!`. Note that `Result<T, E>` itself _does not_
///     implement `UriDisplay<Path>`.
///
/// Rocket implements `UriDisplay<Query>` (but not `UriDisplay<Path>`) for
/// several built-in types.
///
///   * **`Form<T>`, `LenientForm<T>`** _where_ **`T: FromUriParam + FromForm`**
///
///     Uses the implementation of `UriDisplay` for `T::Target`.
///
///     In general, when a type of `Form<T>` is to be displayed as part of a
///     URI's query, it suffices to derive `UriDisplay` for `T`. Note that any
///     type that can be converted into a `T` using [`FromUriParam`] can be used
///     in place of a `Form<T>` in a `uri!` invocation.
///
///   * **`Option<T>`** _where_ **`T: UriDisplay<Query>`**
///
///     If the `Option` is `Some`, uses the implementation of `UriDisplay` for
///     `T`. Otherwise, nothing is rendered.
///
///   * **`Result<T, E>`** _where_ **`T: UriDisplay<Query>`**
///
///     If the `Result` is `Ok`, uses the implementation of `UriDisplay` for
///     `T`. Otherwise, nothing is rendered.
///
/// [`FromUriParam`]: crate::uri::fmt::FromUriParam
///
/// # Deriving
///
/// Manually implementing `UriDisplay` should be done with care. For most use
/// cases, deriving `UriDisplay` will suffice:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # use rocket::http::uri::fmt::{UriDisplay, Query, Path};
/// // Derives `UriDisplay<Query>`
/// #[derive(UriDisplayQuery)]
/// struct User {
///     name: String,
///     age: usize,
/// }
///
/// let user = User { name: "Michael Smith".into(), age: 31 };
/// let uri_string = format!("{}", &user as &dyn UriDisplay<Query>);
/// assert_eq!(uri_string, "name=Michael%20Smith&age=31");
///
/// // Derives `UriDisplay<Path>`
/// #[derive(UriDisplayPath)]
/// struct Name(String);
///
/// let name = Name("Bob Smith".into());
/// let uri_string = format!("{}", &name as &dyn UriDisplay<Path>);
/// assert_eq!(uri_string, "Bob%20Smith");
/// ```
///
/// As long as every field in the structure (or enum) implements `UriDisplay`,
/// the trait can be derived. The implementation calls
/// [`Formatter::write_named_value()`] for every named field and
/// [`Formatter::write_value()`] for every unnamed field. See the
/// [`UriDisplay<Path>`] and [`UriDisplay<Query>`] derive documentation for full
/// details.
///
/// [`Ignorable`]: crate::uri::fmt::Ignorable
/// [`UriDisplay<Path>`]: ../../../derive.UriDisplayPath.html
/// [`UriDisplay<Query>`]: ../../../derive.UriDisplayQuery.html
///
/// # Implementing
///
/// Implementing `UriDisplay` is similar to implementing
/// [`Display`](std::fmt::Display) with the caveat that extra care must be
/// taken to ensure that the written string is URI-safe. As mentioned before, in
/// practice, this means that the string must either be percent-encoded or
/// consist only of characters that are alphanumeric, "-", ".", "_", or "~".
///
/// When manually implementing `UriDisplay` for your types, you should defer to
/// existing implementations of `UriDisplay` as much as possible. In the example
/// below, for instance, `Name`'s implementation defers to `String`'s
/// implementation. To percent-encode a string, use
/// [`Uri::percent_encode()`](crate::uri::Uri::percent_encode()).
///
/// ## Example
///
/// The following snippet consists of a `Name` type that implements both
/// `FromParam` and `UriDisplay<Path>`. The `FromParam` implementation allows
/// `Name` to be used as the target type of a dynamic parameter, while the
/// `UriDisplay` implementation allows URIs to be generated for routes with
/// `Name` as a dynamic path parameter type. Note the custom parsing in the
/// `FromParam` implementation; as a result of this, a custom (reflexive)
/// `UriDisplay` implementation is required.
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// use rocket::request::FromParam;
///
/// struct Name<'r>(&'r str);
///
/// const PREFIX: &str = "name:";
///
/// impl<'r> FromParam<'r> for Name<'r> {
///     type Error = &'r str;
///
///     /// Validates parameters that start with 'name:', extracting the text
///     /// after 'name:' as long as there is at least one character.
///     fn from_param(param: &'r str) -> Result<Self, Self::Error> {
///         if !param.starts_with(PREFIX) || param.len() < (PREFIX.len() + 1) {
///             return Err(param);
///         }
///
///         let real_name = &param[PREFIX.len()..];
///         Ok(Name(real_name))
///     }
/// }
///
/// use std::fmt;
/// use rocket::http::impl_from_uri_param_identity;
/// use rocket::http::uri::fmt::{Formatter, FromUriParam, UriDisplay, Path};
/// use rocket::response::Redirect;
///
/// impl UriDisplay<Path> for Name<'_> {
///     // Writes the raw string `name:`, which is URI-safe, and then delegates
///     // to the `UriDisplay` implementation for `str` which ensures that
///     // string is written in a URI-safe manner. In this case, the string will
///     // be percent encoded.
///     fn fmt(&self, f: &mut Formatter<Path>) -> fmt::Result {
///         f.write_raw("name:")?;
///         UriDisplay::fmt(&self.0, f)
///     }
/// }
///
/// impl_from_uri_param_identity!([Path] ('a) Name<'a>);
///
/// #[get("/name/<name>")]
/// fn redirector(name: Name<'_>) -> Redirect {
///     Redirect::to(uri!(real(name)))
/// }
///
/// #[get("/<name>")]
/// fn real(name: Name<'_>) -> String {
///     format!("Hello, {}!", name.0)
/// }
///
/// let uri = uri!(real(Name("Mike Smith".into())));
/// assert_eq!(uri.path(), "/name:Mike%20Smith");
/// ```
pub trait UriDisplay<P: Part> {
    /// Formats `self` in a URI-safe manner using the given formatter.
    fn fmt(&self, f: &mut Formatter<'_, P>) -> fmt::Result;
}

impl<P: Part> fmt::Display for &dyn UriDisplay<P> {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        UriDisplay::fmt(*self, &mut <Formatter<'_, P>>::new(f))
    }
}

// Direct implementations: these are the leaves of a call to `UriDisplay::fmt`.

/// Percent-encodes the raw string.
impl<P: Part> UriDisplay<P> for str {
    #[inline(always)]
    fn fmt(&self, f: &mut Formatter<'_, P>) -> fmt::Result {
        f.write_raw(RawStr::new(self).percent_encode().as_str())
    }
}

/// Percent-encodes each segment in the path and normalizes separators.
impl UriDisplay<Path> for path::Path {
    fn fmt(&self, f: &mut Formatter<'_, Path>) -> fmt::Result {
        use std::path::Component;

        for component in self.components() {
            match component {
                Component::Prefix(_) | Component::RootDir => continue,
                _ => f.write_value(&component.as_os_str().to_string_lossy())?
            }
        }

        Ok(())
    }
}

macro_rules! impl_with_display {
    ($($T:ty),+ $(,)?) => {$(
        /// This implementation is identical to the `Display` implementation.
        impl<P: Part> UriDisplay<P> for $T  {
            #[inline(always)]
            fn fmt(&self, f: &mut Formatter<'_, P>) -> fmt::Result {
                use std::fmt::Write;
                write!(f, "{}", self)
            }
        }
    )+}
}

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::num::{
    NonZeroIsize, NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128,
    NonZeroUsize, NonZeroU8, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128,
};

// Keep in-sync with the 'FromUriParam' impls.
impl_with_display! {
    i8, i16, i32, i64, i128, isize,
    u8, u16, u32, u64, u128, usize,
    f32, f64, bool,
    IpAddr, Ipv4Addr, Ipv6Addr,
    NonZeroIsize, NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128,
    NonZeroUsize, NonZeroU8, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128,
}

macro_rules! impl_with_string {
    ($($T:ty => $f:expr),+ $(,)?) => {$(
        /// This implementation is identical to a percent-encoded version of the
        /// `Display` implementation.
        impl<P: Part> UriDisplay<P> for $T  {
            #[inline(always)]
            fn fmt(&self, f: &mut Formatter<'_, P>) -> fmt::Result {
                let func: fn(&$T) -> Result<String, fmt::Error> = $f;
                func(self).and_then(|s| s.as_str().fmt(f))
            }
        }
    )+}
}

use std::net::{SocketAddr, SocketAddrV4, SocketAddrV6};

// Keep formats in sync with 'FromFormField' impls.
static DATE_FMT: &[FormatItem<'_>] = format_description!("[year padding:none]-[month]-[day]");
static TIME_FMT: &[FormatItem<'_>] = format_description!("[hour padding:none]:[minute]:[second]");
static DATE_TIME_FMT: &[FormatItem<'_>] =
    format_description!("[year padding:none]-[month]-[day]T[hour padding:none]:[minute]:[second]");

// Keep list in sync with the 'FromUriParam' impls.
impl_with_string! {
    time::Date => |d| d.format(&DATE_FMT).map_err(|_| fmt::Error),
    time::Time => |d| d.format(&TIME_FMT).map_err(|_| fmt::Error),
    time::PrimitiveDateTime => |d| d.format(&DATE_TIME_FMT).map_err(|_| fmt::Error),
    SocketAddr => |a| Ok(a.to_string()),
    SocketAddrV4 => |a| Ok(a.to_string()),
    SocketAddrV6 => |a| Ok(a.to_string()),
}

// These are second level implementations: they all defer to an existing
// implementation. Keep in-sync with `FromUriParam` impls.

/// Percent-encodes the raw string. Defers to `str`.
impl<P: Part> UriDisplay<P> for String {
    #[inline(always)]
    fn fmt(&self, f: &mut Formatter<'_, P>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

/// Percent-encodes the raw string. Defers to `str`.
impl<P: Part> UriDisplay<P> for Cow<'_, str> {
    #[inline(always)]
    fn fmt(&self, f: &mut Formatter<'_, P>) -> fmt::Result {
        self.as_ref().fmt(f)
    }
}

/// Percent-encodes each segment in the path and normalizes separators.
impl UriDisplay<Path> for path::PathBuf {
    #[inline(always)]
    fn fmt(&self, f: &mut Formatter<'_, Path>) -> fmt::Result {
        self.as_path().fmt(f)
    }
}

/// Defers to the `UriDisplay<P>` implementation for `T`.
impl<P: Part, T: UriDisplay<P> + ?Sized> UriDisplay<P> for &T {
    #[inline(always)]
    fn fmt(&self, f: &mut Formatter<'_, P>) -> fmt::Result {
        UriDisplay::fmt(*self, f)
    }
}

/// Defers to the `UriDisplay<P>` implementation for `T`.
impl<P: Part, T: UriDisplay<P> + ?Sized> UriDisplay<P> for &mut T {
    #[inline(always)]
    fn fmt(&self, f: &mut Formatter<'_, P>) -> fmt::Result {
        UriDisplay::fmt(*self, f)
    }
}

/// Defers to the `UriDisplay<Query>` implementation for `T`.
impl<T: UriDisplay<Query>> UriDisplay<Query> for Option<T> {
    #[inline(always)]
    fn fmt(&self, f: &mut Formatter<'_, Query>) -> fmt::Result {
        match self {
            Some(v) => v.fmt(f),
            None => Ok(())
        }
    }
}

/// Defers to the `UriDisplay<Query>` implementation for `T`.
impl<T: UriDisplay<Query>, E> UriDisplay<Query> for Result<T, E> {
    #[inline(always)]
    fn fmt(&self, f: &mut Formatter<'_, Query>) -> fmt::Result {
        match self {
            Ok(v) => v.fmt(f),
            Err(_) => Ok(())
        }
    }
}

impl<T: UriDisplay<Query>> UriDisplay<Query> for Vec<T> {
    fn fmt(&self, f: &mut Formatter<'_, Query>) -> fmt::Result {
        for value in self {
            f.write_value(value)?;
        }

        Ok(())
    }
}

impl<K: UriDisplay<Query>, V: UriDisplay<Query>> UriDisplay<Query> for HashMap<K, V> {
    fn fmt(&self, f: &mut Formatter<'_, Query>) -> fmt::Result {
        use std::fmt::Write;

        let mut field_name = String::with_capacity(8);
        for (i, (key, value)) in self.iter().enumerate() {
            field_name.truncate(0);
            write!(field_name, "k:{}", i)?;
            f.write_named_value(&field_name, key)?;

            field_name.replace_range(..1, "v");
            f.write_named_value(&field_name, value)?;
        }

        Ok(())
    }
}

impl<K: UriDisplay<Query>, V: UriDisplay<Query>> UriDisplay<Query> for BTreeMap<K, V> {
    fn fmt(&self, f: &mut Formatter<'_, Query>) -> fmt::Result {
        use std::fmt::Write;

        let mut field_name = String::with_capacity(8);
        for (i, (key, value)) in self.iter().enumerate() {
            field_name.truncate(0);
            write!(field_name, "k:{}", i)?;
            f.write_named_value(&field_name, key)?;

            field_name.replace_range(..1, "v");
            f.write_named_value(&field_name, value)?;
        }

        Ok(())
    }
}

#[cfg(feature = "uuid")] impl_with_display!(uuid_::Uuid);
#[cfg(feature = "uuid")] crate::impl_from_uri_param_identity!(uuid_::Uuid);

// And finally, the `Ignorable` trait, which has sugar of `_` in the `uri!`
// macro, which expands to a typecheck.

/// Trait implemented by types that can be ignored in `uri!`.
///
/// When a parameter is explicitly ignored in `uri!` by supplying `_` as the
/// parameter's value, that parameter's type is required to implement this
/// trait for the corresponding `Part`.
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// #[get("/item/<id>?<track>")]
/// fn get_item(id: i32, track: Option<u8>) { /* .. */ }
///
/// // Ignore the `track` parameter: `Option<u8>` must be `Ignorable`.
/// uri!(get_item(100, _));
/// uri!(get_item(id = 100, track = _));
///
/// // Provide a value for `track`.
/// uri!(get_item(100, Some(4)));
/// uri!(get_item(id = 100, track = Some(4)));
/// ```
///
/// # Implementations
///
/// Only `Option<T>` and `Result<T, E>` implement this trait. You may implement
/// this trait for your own ignorable types as well:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// use rocket::http::uri::fmt::{Ignorable, Query};
///
/// # struct MyType;
/// impl Ignorable<Query> for MyType { }
/// ```
pub trait Ignorable<P: Part> { }

impl<T> Ignorable<Query> for Option<T> { }
impl<T, E> Ignorable<Query> for Result<T, E> { }

#[doc(hidden)]
pub fn assert_ignorable<P: Part, T: Ignorable<P>>() {  }

#[cfg(test)]
mod uri_display_tests {
    use std::path;
    use crate::uri::fmt::{FromUriParam, UriDisplay};
    use crate::uri::fmt::{Query, Path};

    macro_rules! uri_display {
        (<$P:ident, $Target:ty> $source:expr) => ({
            let tmp = $source;
            let target = <$Target as FromUriParam<$P, _>>::from_uri_param(tmp);
            format!("{}", &target as &dyn UriDisplay<$P>)
        })
    }

    macro_rules! assert_display {
        (<$P:ident, $Target:ty> $source:expr, $expected:expr) => ({
            assert_eq!(uri_display!(<$P, $Target> $source), $expected);
        })
    }

    #[test]
    fn uri_display_encoding() {
        assert_display!(<Query, String> "hello", "hello");
        assert_display!(<Query, String> "hi hi", "hi%20hi");
        assert_display!(<Query, &str> "hi hi", "hi%20hi");
        assert_display!(<Query, &str> &"hi hi", "hi%20hi");
        assert_display!(<Query, usize> 10, "10");
        assert_display!(<Query, u8> 10, "10");
        assert_display!(<Query, i32> 10, "10");
        assert_display!(<Query, isize> 10, "10");

        assert_display!(<Path, String> "hello", "hello");
        assert_display!(<Path, String> "hi hi", "hi%20hi");
        assert_display!(<Path, &str> "hi hi", "hi%20hi");
        assert_display!(<Path, &str> &"hi hi", "hi%20hi");
        assert_display!(<Path, usize> 10, "10");
        assert_display!(<Path, u8> 10, "10");
        assert_display!(<Path, i32> 10, "10");
        assert_display!(<Path, isize> 10, "10");

        assert_display!(<Query, &str> &"hi there", "hi%20there");
        assert_display!(<Query, isize> &10, "10");
        assert_display!(<Query, u8> &10, "10");

        assert_display!(<Path, &str> &"hi there", "hi%20there");
        assert_display!(<Path, isize> &10, "10");
        assert_display!(<Path, u8> &10, "10");

        assert_display!(<Path, Option<&str>> &"hi there", "hi%20there");
        assert_display!(<Path, Option<isize>> &10, "10");
        assert_display!(<Path, Option<u8>> &10, "10");
        assert_display!(<Query, Option<&str>> Some(&"hi there"), "hi%20there");
        assert_display!(<Query, Option<isize>> Some(&10), "10");
        assert_display!(<Query, Option<u8>> Some(&10), "10");

        assert_display!(<Path, Result<&str, usize>> &"hi there", "hi%20there");
        assert_display!(<Path, Result<isize, &str>> &10, "10");
        assert_display!(<Path, Result<u8, String>> &10, "10");
        assert_display!(<Query, Result<&str, usize>> Ok(&"hi there"), "hi%20there");
        assert_display!(<Query, Result<isize, &str>> Ok(&10), "10");
        assert_display!(<Query, Result<u8, String>> Ok(&10), "10");
    }

    #[test]
    fn paths() {
        assert_display!(<Path, path::PathBuf> "hello", "hello");
        assert_display!(<Path, path::PathBuf> "hi there", "hi%20there");
        assert_display!(<Path, path::PathBuf> "hello/world", "hello/world");
        assert_display!(<Path, path::PathBuf> "hello//world", "hello/world");
        assert_display!(<Path, path::PathBuf> "hello/ world", "hello/%20world");

        assert_display!(<Path, path::PathBuf> "hi/wo rld", "hi/wo%20rld");

        assert_display!(<Path, path::PathBuf> &"hi/wo rld", "hi/wo%20rld");
        assert_display!(<Path, path::PathBuf> &"hi there", "hi%20there");
    }

    struct Wrapper<T>(T);

    impl<A, T: FromUriParam<Query, A>> FromUriParam<Query, A> for Wrapper<T> {
        type Target = T::Target;

        #[inline(always)]
        fn from_uri_param(param: A) -> Self::Target {
            T::from_uri_param(param)
        }
    }

    impl FromUriParam<Path, usize> for Wrapper<usize> {
        type Target = usize;

        #[inline(always)]
        fn from_uri_param(param: usize) -> Self::Target {
            param
        }
    }

    #[test]
    fn uri_display_encoding_wrapped() {
        assert_display!(<Query, Option<Wrapper<&str>>> Some(&"hi there"), "hi%20there");
        assert_display!(<Query, Option<Wrapper<&str>>> Some("hi there"), "hi%20there");

        assert_display!(<Query, Option<Wrapper<isize>>> Some(10), "10");
        assert_display!(<Query, Option<Wrapper<usize>>> Some(18), "18");
        assert_display!(<Path, Option<Wrapper<usize>>> 238, "238");

        assert_display!(<Path, Result<Option<Wrapper<usize>>, usize>> 238, "238");
        assert_display!(<Path, Option<Result<Wrapper<usize>, usize>>> 123, "123");
    }

    #[test]
    fn check_ignorables() {
        use crate::uri::fmt::assert_ignorable;

        assert_ignorable::<Query, Option<usize>>();
        assert_ignorable::<Query, Option<Wrapper<usize>>>();
        assert_ignorable::<Query, Result<Wrapper<usize>, usize>>();
        assert_ignorable::<Query, Option<Result<Wrapper<usize>, usize>>>();
        assert_ignorable::<Query, Result<Option<Wrapper<usize>>, usize>>();
    }
}
