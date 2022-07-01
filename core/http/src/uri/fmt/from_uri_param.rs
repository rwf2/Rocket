use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use crate::uri::fmt::UriDisplay;
use crate::uri::fmt::{self, Part};

/// Conversion trait for parameters used in [`uri!`] invocations.
///
/// # Overview
///
/// In addition to implementing [`UriDisplay`], to use a custom type in a `uri!`
/// expression, the `FromUriParam` trait must be implemented. The `UriDisplay`
/// derive automatically generates _identity_ implementations of `FromUriParam`,
/// so in the majority of cases, as with `UriDisplay`, this trait is never
/// implemented manually.
///
/// In the rare case that `UriDisplay` is implemented manually, this trait, too,
/// must be implemented explicitly. In the majority of cases, implementation can
/// be automated. Rocket provides [`impl_from_uri_param_identity`] to generate
/// the _identity_ implementations automatically. For a type `T`, these are:
///
///   * `impl<P: Part> FromUriParam<P, T> for T`
///   * `impl<'x, P: Part> FromUriParam<P, &'x T> for T`
///   * `impl<'x, P: Part> FromUriParam<P, &'x mut T> for T`
///
/// See [`impl_from_uri_param_identity!`](crate::impl_from_uri_param_identity!)
/// for usage details.
///
/// # Code Generation
///
/// This trait is invoked once per expression passed into a [`uri!`] invocation.
/// In particular, for a route URI parameter of type `T` and a user-supplied
/// expression `e` of type `S`, `<T as FromUriParam<S>>::from_uri_param(e)` is
/// invoked. The returned value of type `T::Target` is used in place of the
/// user's value and rendered using its [`UriDisplay`] implementation.
///
/// This trait allows types that differ from the route URI parameter's types to
/// be used in their place at no cost. For instance, the following
/// implementation, provided by Rocket, allows an `&str` to be used in a `uri!`
/// invocation for route URI parameters declared as `String`:
///
/// ```rust
/// # extern crate rocket;
/// # use rocket::http::uri::fmt::{FromUriParam, Part};
/// # struct S;
/// # type String = S;
/// impl<'a, P: Part> FromUriParam<P, &'a str> for String {
///     type Target = &'a str;
/// #   fn from_uri_param(s: &'a str) -> Self::Target { "hi" }
/// }
/// ```
///
/// Because the [`FromUriParam::Target`] type is the same as the input type, the
/// conversion is a no-op and free of cost, allowing an `&str` to be used in
/// place of a `String` without penalty.
///
/// # Provided Implementations
///
/// The following types have _identity_ implementations:
///
///    * `String`, `i8`, `i16`, `i32`, `i64`, `i128`, `isize`, `u8`, `u16`,
///      `u32`, `u64`, `u128`, `usize`, `f32`, `f64`, `bool`, `IpAddr`,
///      `Ipv4Addr`, `Ipv6Addr`, `&str`, `Cow<str>`
///
/// The following types have _identity_ implementations _only in [`Path`]_:
///
///   * `&Path`, `PathBuf`
///
/// The following types have _identity_ implementations _only in [`Query`]_:
///
///   * `Option<T>`, `Result<T, E>`
///
/// The following conversions are implemented for both paths and queries,
/// allowing a value of the type on the left to be used when a type on the right
/// is expected by a route:
///
///   * `&str` to `String`
///   * `String` to `&str`
///   * `T` to `Form<T>`
///
/// The following conversions are implemented _only in [`Path`]_:
///
///   * `&str` to `&Path`
///   * `&str` to `PathBuf`
///   * `PathBuf` to `&Path`
///   * `T` to `Option<T>`
///   * `T` to `Result<T, E>`
///
/// The following conversions are implemented _only in [`Query`]_:
///
///   * `Option<T>` to `Result<T, E>` (for any `E`)
///   * `Result<T, E>` to `Option<T>` (for any `E`)
///
/// See [Foreign Impls](#foreign-impls) for all provided implementations.
///
/// # Implementing
///
/// This trait should only be implemented when you'd like to allow a type
/// different from the route's declared type to be used in its place in a `uri!`
/// invocation. For instance, if the route has a type of `T` and you'd like to
/// use a type of `S` in a `uri!` invocation, you'd implement `FromUriParam<P,
/// T> for S` where `P` is `Path` for conversions valid in the path part of a
/// URI, `Uri` for conversions valid in the query part of a URI, or `P: Part`
/// when a conversion is valid in either case.
///
/// This is typically only warranted for owned-value types with corresponding
/// reference types: `String` and `&str`, for instance. In this case, it's
/// desirable to allow an `&str` to be used in place of a `String`.
///
/// When implementing `FromUriParam`, be aware that Rocket will use the
/// [`UriDisplay`] implementation of [`FromUriParam::Target`], _not_ of the
/// source type. Incorrect implementations can result in creating unsafe URIs.
///
/// # Example
///
/// The following example implements `FromUriParam<Query, (&str, &str)>` for a
/// `User` type. The implementation allows an `(&str, &str)` type to be used in
/// a `uri!` invocation where a `User` type is expected in the query part of the
/// URI.
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// use std::fmt;
///
/// use rocket::http::uri::fmt::{Formatter, UriDisplay, FromUriParam, Query};
///
/// #[derive(FromForm)]
/// struct User<'a> {
///     name: &'a str,
///     nickname: String,
/// }
///
/// impl UriDisplay<Query> for User<'_> {
///     fn fmt(&self, f: &mut Formatter<Query>) -> fmt::Result {
///         f.write_named_value("name", &self.name)?;
///         f.write_named_value("nickname", &self.nickname)
///     }
/// }
///
/// impl<'a, 'b> FromUriParam<Query, (&'a str, &'b str)> for User<'a> {
///     type Target = User<'a>;
///
///     fn from_uri_param((name, nickname): (&'a str, &'b str)) -> User<'a> {
///         User { name: name.into(), nickname: nickname.to_string() }
///     }
/// }
/// ```
///
/// With these implementations, the following typechecks:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # use std::fmt;
/// # use rocket::http::uri::fmt::{Formatter, UriDisplay, FromUriParam, Query};
/// #
/// # #[derive(FromForm)]
/// # struct User<'a> { name: &'a str, nickname: String, }
/// #
/// # impl UriDisplay<Query> for User<'_> {
/// #     fn fmt(&self, f: &mut Formatter<Query>) -> fmt::Result {
/// #         f.write_named_value("name", &self.name)?;
/// #         f.write_named_value("nickname", &self.nickname)
/// #     }
/// # }
/// #
/// # impl<'a, 'b> FromUriParam<Query, (&'a str, &'b str)> for User<'a> {
/// #     type Target = User<'a>;
/// #     fn from_uri_param((name, nickname): (&'a str, &'b str)) -> User<'a> {
/// #         User { name: name.into(), nickname: nickname.to_string() }
/// #     }
/// # }
/// #
/// #[post("/<name>?<user..>")]
/// fn some_route(name: &str, user: User<'_>)  { /* .. */ }
///
/// let uri = uri!(some_route(name = "hey", user = ("Robert Mike", "Bob")));
/// assert_eq!(uri.path(), "/hey");
/// assert_eq!(uri.query().unwrap(), "name=Robert%20Mike&nickname=Bob");
/// ```
///
/// [`uri!`]: rocket::uri
/// [`FromUriParam::Target`]: crate::uri::fmt::FromUriParam::Target
/// [`Path`]: crate::uri::fmt::Path
/// [`Query`]: crate::uri::fmt::Query
pub trait FromUriParam<P: Part, T> {
    /// The resulting type of this conversion.
    type Target: UriDisplay<P>;

    /// Converts a value of type `T` into a value of type `Self::Target`. The
    /// resulting value of type `Self::Target` will be rendered into a URI using
    /// its [`UriDisplay`] implementation.
    fn from_uri_param(param: T) -> Self::Target;
}

#[doc(hidden)]
#[macro_export(local_inner_macros)]
macro_rules! impl_conversion_ref {
    ($(($($l:tt)+) $A:ty => $B:ty),* $(,)?) => (
        impl_conversion_ref!(@_ $(($($l)+,) $A => $B),*);
    );

    ($($A:ty => $B:ty),* $(,)?) => (
        impl_conversion_ref!(@_ $(() $A => $B),*);
    );

    (@_ $(($($l:tt)*) $A:ty => $B:ty),* $(,)?) => ($(
        impl_conversion_ref!([P] ($($l)* P: $crate::uri::fmt::Part) $A => $B);
    )*);

    ($([$P:ty] ($($l:tt)*) $A:ty => $B:ty),* $(,)?) => ($(
        impl_conversion_ref!(@_ [$P] ($($l)*) $A => $B);
        impl_conversion_ref!(@_ [$P] ('x, $($l)*) &'x $A => $B);
        impl_conversion_ref!(@_ [$P] ('x, $($l)*) &'x mut $A => $B);
    )*);

    ($([$P:ty] $A:ty => $B:ty),* $(,)?) => ( impl_conversion_ref!($([$P] () $A => $B),*););

    (@_ [$P:ty] ($($l:tt)*) $A:ty => $B:ty) => (
        impl<$($l)*> $crate::uri::fmt::FromUriParam<$P, $A> for $B {
            type Target = $A;
            #[inline(always)] fn from_uri_param(param: $A) -> $A { param }
        }
    );
}

/// Macro to automatically generate _identity_ [`FromUriParam`] trait
/// implementations.
///
/// For a type `T`, the _identity_ implementations of `FromUriParam` are:
///
///   * `impl<P: Part> FromUriParam<P, T> for T`
///   * `impl<'x> FromUriParam<P, &'x T> for T`
///   * `impl<'x> FromUriParam<P, &'x mut T> for T`
///
/// where `P` is one of:
///
///   * `P: Part` (the generic `P`)
///   * [`Path`]
///   * [`Query`]
///
/// This macro can be invoked in four ways:
///
///   1. `impl_from_uri_param_identity!(Type);`
///
///      Generates the three _identity_ implementations for the generic `P`.
///
///      * Example: `impl_from_uri_param_identity!(MyType);`
///      * Generates: `impl<P: Part> FromUriParam<P, _> for MyType { ... }`
///
///   2. `impl_from_uri_param_identity!((generics*) Type);`
///
///      Generates the three _identity_ implementations for the generic `P`,
///      adding the tokens `generics` to the `impl` generics of the generated
///      implementation.
///
///      * Example: `impl_from_uri_param_identity!(('a) MyType<'a>);`
///      * Generates: `impl<'a, P: Part> FromUriParam<P, _> for MyType<'a> { ... }`
///
///   3. `impl_from_uri_param_identity!([Part] Type);`
///
///      Generates the three _identity_ implementations for the `Part`
///      `Part`, where `Part` is a path to [`Path`] or [`Query`].
///
///      * Example: `impl_from_uri_param_identity!([Path] MyType);`
///      * Generates: `impl FromUriParam<Path, _> for MyType { ... }`
///
///   4. `impl_from_uri_param_identity!([Part] (generics*) Type);`
///
///      See 2 and 3.
///
///      * Example: `impl_from_uri_param_identity!([Path] ('a) MyType<'a>);`
///      * Generates: `impl<'a> FromUriParam<Path, _> for MyType<'a> { ... }`
///
/// [`FromUriParam`]: crate::uri::fmt::FromUriParam
/// [`Path`]: crate::uri::fmt::Path
/// [`Query`]: crate::uri::fmt::Query
#[macro_export(local_inner_macros)]
macro_rules! impl_from_uri_param_identity {
    ($(($($l:tt)*) $T:ty),* $(,)?) => ($( impl_conversion_ref!(($($l)*) $T => $T); )*);
    ($([$P:ty] ($($l:tt)*) $T:ty),* $(,)?) => ($( impl_conversion_ref!([$P] ($($l)*) $T => $T); )*);
    ($([$P:ty] $T:ty),* $(,)?) => ($( impl_conversion_ref!([$P] $T => $T); )*);
    ($($T:ty),* $(,)?) => ($( impl_conversion_ref!($T => $T); )*);
}

use std::borrow::Cow;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::num::{
    NonZeroIsize, NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128,
    NonZeroUsize, NonZeroU8, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128,
};

impl_from_uri_param_identity! {
    String,
    i8, i16, i32, i64, i128, isize,
    u8, u16, u32, u64, u128, usize,
    f32, f64, bool,
    IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6,
    NonZeroIsize, NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128,
    NonZeroUsize, NonZeroU8, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128,
    time::Date, time::Time, time::PrimitiveDateTime,
}

impl_from_uri_param_identity! {
    ('a) &'a str,
    ('a) Cow<'a, str>
}

impl_conversion_ref! {
    ('a) &'a str => String,

    ('a) String => &'a str
}

impl_from_uri_param_identity!([fmt::Path] ('a) &'a Path);
impl_from_uri_param_identity!([fmt::Path] PathBuf);

impl_conversion_ref! {
    [fmt::Path] ('a) &'a Path => PathBuf,
    [fmt::Path] ('a) PathBuf => &'a Path
}

/// A no cost conversion allowing an `&str` to be used in place of a `PathBuf`.
impl<'a> FromUriParam<fmt::Path, &'a str> for PathBuf {
    type Target = &'a Path;

    #[inline(always)]
    fn from_uri_param(param: &'a str) -> &'a Path {
        Path::new(param)
    }
}

/// A no cost conversion allowing an `&&str` to be used in place of a `PathBuf`.
impl<'a, 'b> FromUriParam<fmt::Path, &'a &'b str> for PathBuf {
    type Target = &'b Path;

    #[inline(always)]
    fn from_uri_param(param: &'a &'b str) -> &'b Path {
        Path::new(*param)
    }
}

/// A no cost conversion allowing any `T` to be used in place of an `Option<T>`.
impl<A, T: FromUriParam<fmt::Path, A>> FromUriParam<fmt::Path, A> for Option<T> {
    type Target = T::Target;

    #[inline(always)]
    fn from_uri_param(param: A) -> Self::Target {
        T::from_uri_param(param)
    }
}

/// A no cost conversion allowing `T` to be used in place of an `Result<T, E>`.
impl<A, E, T: FromUriParam<fmt::Path, A>> FromUriParam<fmt::Path, A> for Result<T, E> {
    type Target = T::Target;

    #[inline(always)]
    fn from_uri_param(param: A) -> Self::Target {
        T::from_uri_param(param)
    }
}

impl<A, T: FromUriParam<fmt::Query, A>> FromUriParam<fmt::Query, Option<A>> for Option<T> {
    type Target = Option<T::Target>;

    #[inline(always)]
    fn from_uri_param(param: Option<A>) -> Self::Target {
        param.map(T::from_uri_param)
    }
}

impl<A, E, T: FromUriParam<fmt::Query, A>> FromUriParam<fmt::Query, Option<A>> for Result<T, E> {
    type Target = Option<T::Target>;

    #[inline(always)]
    fn from_uri_param(param: Option<A>) -> Self::Target {
        param.map(T::from_uri_param)
    }
}

impl<A, E, T: FromUriParam<fmt::Query, A>> FromUriParam<fmt::Query, Result<A, E>> for Result<T, E> {
    type Target = Result<T::Target, E>;

    #[inline(always)]
    fn from_uri_param(param: Result<A, E>) -> Self::Target {
        param.map(T::from_uri_param)
    }
}

impl<A, E, T: FromUriParam<fmt::Query, A>> FromUriParam<fmt::Query, Result<A, E>> for Option<T> {
    type Target = Result<T::Target, E>;

    #[inline(always)]
    fn from_uri_param(param: Result<A, E>) -> Self::Target {
        param.map(T::from_uri_param)
    }
}

macro_rules! impl_map_conversion {
    ($From:ident => $To:ident) => (
        impl<K, V, A, B> FromUriParam<fmt::Query, $From<A, B>> for $To<K, V>
            where A: UriDisplay<fmt::Query>, K: FromUriParam<fmt::Query, A>,
                  B: UriDisplay<fmt::Query>, V: FromUriParam<fmt::Query, B>
        {
            type Target = $From<A, B>;

            #[inline(always)]
            fn from_uri_param(param: $From<A, B>) -> Self::Target {
                param
            }
        }
    );

    (& $([$mut:tt])? $From:ident => $To:ident) => (
        impl<'a, K, V, A, B> FromUriParam<fmt::Query, &'a $($mut)? $From<A, B>> for $To<K, V>
            where A: UriDisplay<fmt::Query>, K: FromUriParam<fmt::Query, A>,
                  B: UriDisplay<fmt::Query>, V: FromUriParam<fmt::Query, B>
        {
            type Target = &'a $From<A, B>;

            #[inline(always)]
            fn from_uri_param(param: &'a $($mut)? $From<A, B>) -> Self::Target {
                param
            }
        }
    );
}

impl_map_conversion!(HashMap => HashMap);
impl_map_conversion!(HashMap => BTreeMap);
impl_map_conversion!(BTreeMap => BTreeMap);
impl_map_conversion!(BTreeMap => HashMap);

impl_map_conversion!(&HashMap => HashMap);
impl_map_conversion!(&HashMap => BTreeMap);
impl_map_conversion!(&BTreeMap => BTreeMap);
impl_map_conversion!(&BTreeMap => HashMap);

impl_map_conversion!(&[mut] HashMap => HashMap);
impl_map_conversion!(&[mut] HashMap => BTreeMap);
impl_map_conversion!(&[mut] BTreeMap => BTreeMap);
impl_map_conversion!(&[mut] BTreeMap => HashMap);
