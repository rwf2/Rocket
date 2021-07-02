use std::fmt::{self, Write};
use std::marker::PhantomData;
use std::borrow::Cow;

use smallvec::SmallVec;

use crate::uri::{Absolute, Origin, Reference};
use crate::uri::fmt::{UriDisplay, Part, Path, Query, Kind};

/// A struct used to format strings for [`UriDisplay`].
///
/// # Marker Generic: `Formatter<Path>` vs. `Formatter<Query>`
///
/// Like [`UriDisplay`], the [`Part`] parameter `P` in `Formatter<P>` must be
/// either [`Path`] or [`Query`] resulting in either `Formatter<Path>` or
/// `Formatter<Query>`. The `Path` version is used when formatting parameters
/// in the path part of the URI while the `Query` version is used when
/// formatting parameters in the query part of the URI. The
/// [`write_named_value()`] method is only available to `UriDisplay<Query>`.
///
/// # Overview
///
/// A mutable version of this struct is passed to [`UriDisplay::fmt()`]. This
/// struct properly formats series of values for use in URIs. In particular,
/// this struct applies the following transformations:
///
///   * When **multiple values** are written, they are separated by `/` for
///     `Path` types and `&` for `Query` types.
///
/// Additionally, for `Formatter<Query>`:
///
///   * When a **named value** is written with [`write_named_value()`], the name
///     is written out, followed by a `=`, followed by the value.
///
///   * When **nested named values** are written, typically by passing a value
///     to [`write_named_value()`] whose implementation of `UriDisplay` also
///     calls `write_named_vlaue()`, the nested names are joined by a `.`,
///     written out followed by a `=`, followed by the value.
///
/// # Usage
///
/// Usage is fairly straightforward:
///
///   * For every _named value_ you wish to emit, call [`write_named_value()`].
///   * For every _unnamed value_ you wish to emit, call [`write_value()`].
///   * To write a string directly, call [`write_raw()`].
///
/// The `write_named_value` method automatically prefixes the `name` to the
/// written value and, along with `write_value` and `write_raw`, handles nested
/// calls to `write_named_value` automatically, prefixing names when necessary.
/// Unlike the other methods, `write_raw` does _not_ prefix any nested names
/// every time it is called. Instead, it only prefixes the _first_ time it is
/// called, after a call to `write_named_value` or `write_value`, or after a
/// call to [`refresh()`].
///
/// # Example
///
/// The following example uses all of the `write` methods in a varied order to
/// display the semantics of `Formatter<Query>`. Note that `UriDisplay` should
/// rarely be implemented manually, preferring to use the derive, and that this
/// implementation is purely demonstrative.
///
/// ```rust
/// # extern crate rocket;
/// use std::fmt;
///
/// use rocket::http::uri::fmt::{Formatter, UriDisplay, Query};
///
/// struct Outer {
///     value: Inner,
///     another: usize,
///     extra: usize
/// }
///
/// struct Inner {
///     value: usize,
///     extra: usize
/// }
///
/// impl UriDisplay<Query> for Outer {
///     fn fmt(&self, f: &mut Formatter<Query>) -> fmt::Result {
///         f.write_named_value("outer_field", &self.value)?;
///         f.write_named_value("another", &self.another)?;
///         f.write_raw("out")?;
///         f.write_raw("side")?;
///         f.write_value(&self.extra)
///     }
/// }
///
/// impl UriDisplay<Query> for Inner {
///     fn fmt(&self, f: &mut Formatter<Query>) -> fmt::Result {
///         f.write_named_value("inner_field", &self.value)?;
///         f.write_value(&self.extra)?;
///         f.write_raw("inside")
///     }
/// }
///
/// let inner = Inner { value: 0, extra: 1 };
/// let outer = Outer { value: inner, another: 2, extra: 3 };
/// let uri_string = format!("{}", &outer as &dyn UriDisplay<Query>);
/// assert_eq!(uri_string, "outer_field.inner_field=0&\
///                         outer_field=1&\
///                         outer_field=inside&\
///                         another=2&\
///                         outside&\
///                         3");
/// ```
///
/// Note that you can also use the `write!` macro to write directly to the
/// formatter as long as the [`std::fmt::Write`] trait is in scope. Internally,
/// the `write!` macro calls [`write_raw()`], so care must be taken to ensure
/// that the written string is URI-safe.
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// use std::fmt::{self, Write};
///
/// use rocket::http::uri::fmt::{UriDisplay, Formatter, Part, Path, Query};
///
/// pub struct Complex(u8, u8);
///
/// impl<P: Part> UriDisplay<P> for Complex {
///     fn fmt(&self, f: &mut Formatter<P>) -> fmt::Result {
///         write!(f, "{}+{}", self.0, self.1)
///     }
/// }
///
/// let uri_string = format!("{}", &Complex(42, 231) as &dyn UriDisplay<Path>);
/// assert_eq!(uri_string, "42+231");
///
/// #[derive(UriDisplayQuery)]
/// struct Message {
///     number: Complex,
/// }
///
/// let message = Message { number: Complex(42, 47) };
/// let uri_string = format!("{}", &message as &dyn UriDisplay<Query>);
/// assert_eq!(uri_string, "number=42+47");
/// ```
///
/// [`write_named_value()`]: Formatter::write_value()
/// [`write_value()`]: Formatter::write_value()
/// [`write_raw()`]: Formatter::write_raw()
/// [`refresh()`]: Formatter::refresh()
pub struct Formatter<'i, P: Part> {
    prefixes: SmallVec<[&'static str; 3]>,
    inner: &'i mut (dyn Write + 'i),
    previous: bool,
    fresh: bool,
    _marker: PhantomData<P>,
}

impl<'i, P: Part> Formatter<'i, P> {
    #[inline(always)]
    pub(crate) fn new(inner: &'i mut (dyn Write + 'i)) -> Self {
        Formatter {
            inner,
            prefixes: SmallVec::new(),
            previous: false,
            fresh: true,
            _marker: PhantomData,
        }
    }

    #[inline(always)]
    fn refreshed<F: FnOnce(&mut Self) -> fmt::Result>(&mut self, f: F) -> fmt::Result {
        self.refresh();
        let result = f(self);
        self.refresh();
        result
    }

    /// Writes `string` to `self`.
    ///
    /// If `self` is _fresh_ (after a call to other `write_` methods or
    /// [`refresh()`]), prefixes any names and adds separators as necessary.
    ///
    /// This method is called by the `write!` macro.
    ///
    /// [`refresh()`]: Formatter::refresh()
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use std::fmt;
    ///
    /// use rocket::http::uri::fmt::{Formatter, UriDisplay, Part, Path};
    ///
    /// struct Foo;
    ///
    /// impl<P: Part> UriDisplay<P> for Foo {
    ///     fn fmt(&self, f: &mut Formatter<P>) -> fmt::Result {
    ///         f.write_raw("f")?;
    ///         f.write_raw("o")?;
    ///         f.write_raw("o")
    ///     }
    /// }
    ///
    /// let foo = Foo;
    /// let uri_string = format!("{}", &foo as &dyn UriDisplay<Path>);
    /// assert_eq!(uri_string, "foo");
    /// ```
    pub fn write_raw<S: AsRef<str>>(&mut self, string: S) -> fmt::Result {
        // This implementation is a bit of a lie to the type system. Instead of
        // implementing this twice, one for <Path> and again for <Query>, we do
        // this once here. This is okay since we know that this handles the
        // cases for both Path and Query, and doing it this way allows us to
        // keep the uri part generic _generic_ in other implementations that use
        // `write_raw`.
        if self.fresh {
            if self.previous {
                self.inner.write_char(P::DELIMITER)?;
            }

            if P::KIND == Kind::Query && !self.prefixes.is_empty() {
                for (i, prefix) in self.prefixes.iter().enumerate() {
                    if i != 0 { self.inner.write_char('.')? }
                    self.inner.write_str(prefix)?;
                }

                self.inner.write_str("=")?;
            }
        }

        self.fresh = false;
        self.previous = true;
        self.inner.write_str(string.as_ref())
    }

    /// Writes the unnamed value `value`. Any nested names are prefixed as
    /// necessary.
    ///
    /// Refreshes `self` before and after the value is written.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use std::fmt;
    ///
    /// use rocket::http::uri::fmt::{Formatter, UriDisplay, Part, Path, Query};
    ///
    /// struct Foo(usize);
    ///
    /// impl<P: Part> UriDisplay<P> for Foo {
    ///     fn fmt(&self, f: &mut Formatter<P>) -> fmt::Result {
    ///         f.write_value(&self.0)
    ///     }
    /// }
    ///
    /// let foo = Foo(123);
    ///
    /// let uri_string = format!("{}", &foo as &dyn UriDisplay<Path>);
    /// assert_eq!(uri_string, "123");
    ///
    /// let uri_string = format!("{}", &foo as &dyn UriDisplay<Query>);
    /// assert_eq!(uri_string, "123");
    /// ```
    #[inline]
    pub fn write_value<T: UriDisplay<P>>(&mut self, value: T) -> fmt::Result {
        self.refreshed(|f| UriDisplay::fmt(&value, f))
    }

    /// Refreshes the formatter.
    ///
    /// After refreshing, [`write_raw()`] will prefix any nested names as well
    /// as insert a separator.
    ///
    /// [`write_raw()`]: Formatter::write_raw()
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use std::fmt;
    ///
    /// use rocket::http::uri::fmt::{Formatter, UriDisplay, Query, Path};
    ///
    /// struct Foo;
    ///
    /// impl UriDisplay<Query> for Foo {
    ///     fn fmt(&self, f: &mut Formatter<Query>) -> fmt::Result {
    ///         f.write_raw("a")?;
    ///         f.write_raw("raw")?;
    ///         f.refresh();
    ///         f.write_raw("format")
    ///     }
    /// }
    ///
    /// let uri_string = format!("{}", &Foo as &dyn UriDisplay<Query>);
    /// assert_eq!(uri_string, "araw&format");
    ///
    /// impl UriDisplay<Path> for Foo {
    ///     fn fmt(&self, f: &mut Formatter<Path>) -> fmt::Result {
    ///         f.write_raw("a")?;
    ///         f.write_raw("raw")?;
    ///         f.refresh();
    ///         f.write_raw("format")
    ///     }
    /// }
    ///
    /// let uri_string = format!("{}", &Foo as &dyn UriDisplay<Path>);
    /// assert_eq!(uri_string, "araw/format");
    ///
    /// #[derive(UriDisplayQuery)]
    /// struct Message {
    ///     inner: Foo,
    /// }
    ///
    /// let msg = Message { inner: Foo };
    /// let uri_string = format!("{}", &msg as &dyn UriDisplay<Query>);
    /// assert_eq!(uri_string, "inner=araw&inner=format");
    /// ```
    #[inline(always)]
    pub fn refresh(&mut self) {
        self.fresh = true;
    }
}

impl Formatter<'_, Query> {
    fn with_prefix<F>(&mut self, prefix: &str, f: F) -> fmt::Result
        where F: FnOnce(&mut Self) -> fmt::Result
    {

        struct PrefixGuard<'f, 'i>(&'f mut Formatter<'i, Query>);

        impl<'f, 'i> PrefixGuard<'f, 'i> {
            fn new(prefix: &str, f: &'f mut Formatter<'i, Query>) -> Self {
                // SAFETY: The `prefix` string is pushed in a `StackVec` for use
                // by recursive (nested) calls to `write_raw`. The string is
                // pushed in `PrefixGuard` here and then popped in `Drop`.
                // `prefixes` is modified nowhere else, and no concrete-lifetime
                // strings leak from the the vector. As a result, it is
                // impossible for a `prefix` to be accessed incorrectly as:
                //
                //   * Rust _guarantees_ `prefix` is valid for this method
                //   * `prefix` is only reachable while this method's stack is
                //     active because it is unconditionally popped before this
                //     method returns via `PrefixGuard::drop()`.
                //   * should a panic occur in `f()`, `PrefixGuard::drop()` is
                //     still called (or the program aborts), ensuring `prefix`
                //     is no longer in `prefixes` and thus inaccessible.
                //   * thus, at any point `prefix` is reachable, it is valid
                //
                // Said succinctly: `prefixes` shadows a subset of the
                // `with_prefix` stack, making it reachable to other code.
                let prefix = unsafe { std::mem::transmute(prefix) };
                f.prefixes.push(prefix);
                PrefixGuard(f)
            }
        }

        impl Drop for PrefixGuard<'_, '_> {
            fn drop(&mut self) {
                self.0.prefixes.pop();
            }
        }

        f(&mut PrefixGuard::new(prefix, self).0)
    }

    /// Writes the named value `value` by prefixing `name` followed by `=` to
    /// the value. Any nested names are also prefixed as necessary.
    ///
    /// Refreshes `self` before the name is written and after the value is
    /// written.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use std::fmt;
    ///
    /// use rocket::http::uri::fmt::{Formatter, UriDisplay, Query};
    ///
    /// struct Foo {
    ///     name: usize
    /// }
    ///
    /// // Note: This is identical to what #[derive(UriDisplayQuery)] would
    /// // generate! In practice, _always_ use the derive.
    /// impl UriDisplay<Query> for Foo {
    ///     fn fmt(&self, f: &mut Formatter<Query>) -> fmt::Result {
    ///         f.write_named_value("name", &self.name)
    ///     }
    /// }
    ///
    /// let foo = Foo { name: 123 };
    /// let uri_string = format!("{}", &foo as &dyn UriDisplay<Query>);
    /// assert_eq!(uri_string, "name=123");
    /// ```
    #[inline]
    pub fn write_named_value<T: UriDisplay<Query>>(&mut self, name: &str, value: T) -> fmt::Result {
        self.refreshed(|f| f.with_prefix(name, |f| f.write_value(value)))
    }
}

impl<P: Part> fmt::Write for Formatter<'_, P> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_raw(s)
    }
}

// Used by code generation.
#[doc(hidden)]
pub enum UriArgumentsKind<A> {
    Static(&'static str),
    Dynamic(A)
}

// Used by code generation.
#[doc(hidden)]
pub enum UriQueryArgument<'a> {
    Raw(&'a str),
    NameValue(&'a str, &'a dyn UriDisplay<Query>),
    Value(&'a dyn UriDisplay<Query>)
}

/// No prefix at all.
#[doc(hidden)]
pub struct Void;

// Used by code generation.
#[doc(hidden)]
pub trait ValidRoutePrefix {
    type Output;

    fn append(self, path: Cow<'static, str>, query: Option<Cow<'static, str>>) -> Self::Output;
}

impl<'a> ValidRoutePrefix for Origin<'a> {
    type Output = Self;

    fn append(self, path: Cow<'static, str>, query: Option<Cow<'static, str>>) -> Self::Output {
        // No-op if `self` is already normalzied.
        let mut prefix = self.into_normalized();
        prefix.clear_query();

        if prefix.path() == "/" {
            // Avoid a double `//` to start.
            return Origin::new(path, query);
        } else if path == "/" {
            // Appending path to `/` is a no-op, but append any query.
            prefix.set_query(query);
            return prefix;
        }

        Origin::new(format!("{}{}", prefix.path(), path), query)
    }
}

impl<'a> ValidRoutePrefix for Absolute<'a> {
    type Output = Self;

    fn append(self, path: Cow<'static, str>, query: Option<Cow<'static, str>>) -> Self::Output {
        // No-op if `self` is already normalzied.
        let mut prefix = self.into_normalized();
        prefix.clear_query();

        if prefix.authority().is_some() {
            // The prefix is normalized. Appending a `/` is a no-op.
            if path == "/" {
                prefix.set_query(query);
                return prefix;
            }
        }

        // In these cases, appending `path` would be a no-op or worse.
        if prefix.path().is_empty() || prefix.path() == "/" {
            prefix.set_path(path);
            prefix.set_query(query);
            return prefix;
        }

        if path == "/" {
            prefix.set_query(query);
            return prefix;
        }

        prefix.set_path(format!("{}{}", prefix.path(), path));
        prefix.set_query(query);
        prefix
    }
}

// `Self` is a valid suffix for `T`.
#[doc(hidden)]
pub trait ValidRouteSuffix<T> {
    type Output;

    fn prepend(self, prefix: T) -> Self::Output;
}

impl<'a> ValidRouteSuffix<Origin<'a>> for Reference<'a> {
    type Output = Self;

    fn prepend(self, prefix: Origin<'a>) -> Self::Output {
        Reference::from(prefix).with_query_fragment_of(self)
    }
}

impl<'a> ValidRouteSuffix<Absolute<'a>> for Reference<'a> {
    type Output = Self;

    fn prepend(self, prefix: Absolute<'a>) -> Self::Output {
        Reference::from(prefix).with_query_fragment_of(self)
    }
}

impl<'a> ValidRouteSuffix<Origin<'a>> for Absolute<'a> {
    type Output = Origin<'a>;

    fn prepend(self, mut prefix: Origin<'a>) -> Self::Output {
        if let Some(query) = self.query {
            if prefix.query().is_none() {
                prefix.set_query(query.value.into_concrete(&self.source));
            }
        }

        prefix
    }
}

impl<'a> ValidRouteSuffix<Absolute<'a>> for Absolute<'a> {
    type Output = Self;

    fn prepend(self, mut prefix: Absolute<'a>) -> Self::Output {
        if let Some(query) = self.query {
            if prefix.query().is_none() {
                prefix.set_query(query.value.into_concrete(&self.source));
            }
        }

        prefix
    }
}

// Used by code generation.
#[doc(hidden)]
pub struct RouteUriBuilder {
    pub path: Cow<'static, str>,
    pub query: Option<Cow<'static, str>>,
}

// Used by code generation.
#[doc(hidden)]
pub struct PrefixedRouteUri<T>(T);

// Used by code generation.
#[doc(hidden)]
pub struct SuffixedRouteUri<T>(T);

// Used by code generation.
#[doc(hidden)]
impl RouteUriBuilder {
    /// Create a new `RouteUriBuilder` with the given path/query args.
    pub fn new(
        path_args: UriArgumentsKind<&[&dyn UriDisplay<Path>]>,
        query_args: Option<UriArgumentsKind<&[UriQueryArgument<'_>]>>,
    ) -> Self {
        use self::{UriArgumentsKind::*, UriQueryArgument::*};

        let path: Cow<'static, str> = match path_args {
            Static(path) => path.into(),
            Dynamic(args) => {
                let mut string = String::from("/");
                let mut formatter = Formatter::<Path>::new(&mut string);
                for value in args {
                    let _ = formatter.write_value(value);
                }

                string.into()
            }
        };

        let query: Option<Cow<'_, str>> = match query_args {
            None => None,
            Some(Static(query)) => Some(query.into()),
            Some(Dynamic(args)) => {
                let mut string = String::new();
                let mut f = Formatter::<Query>::new(&mut string);
                for arg in args {
                    let _ = match arg {
                        Raw(v) => f.write_raw(v),
                        NameValue(n, v) => f.write_named_value(n, v),
                        Value(v) => f.write_value(v),
                    };
                }

                (!string.is_empty()).then(|| string.into())
            }
        };

        RouteUriBuilder { path, query }
    }

    pub fn with_prefix<P: ValidRoutePrefix>(self, p: P) -> PrefixedRouteUri<P::Output> {
        PrefixedRouteUri(p.append(self.path, self.query))
    }

    pub fn with_suffix<S>(self, suffix: S) -> SuffixedRouteUri<S::Output>
        where S: ValidRouteSuffix<Origin<'static>>
    {
        SuffixedRouteUri(suffix.prepend(self.render()))
    }

    pub fn render(self) -> Origin<'static> {
        Origin::new(self.path, self.query)
    }
}

#[doc(hidden)]
impl<T> PrefixedRouteUri<T> {
    pub fn with_suffix<S: ValidRouteSuffix<T>>(self, suffix: S) -> SuffixedRouteUri<S::Output> {
        SuffixedRouteUri(suffix.prepend(self.0))
    }

    pub fn render(self) -> T {
        self.0
    }
}

#[doc(hidden)]
impl<T> SuffixedRouteUri<T> {
    pub fn render(self) -> T {
        self.0
    }
}

// See https://github.com/SergioBenitez/Rocket/issues/1534.
#[cfg(test)]
mod prefix_soundness_test {
    use crate::uri::fmt::{Formatter, UriDisplay, Query};

    struct MyValue;

    impl UriDisplay<Query> for MyValue {
        fn fmt(&self, _f: &mut Formatter<'_, Query>) -> std::fmt::Result {
            panic!()
        }
    }

    struct MyDisplay;

    impl UriDisplay<Query> for MyDisplay {
        fn fmt(&self, formatter: &mut Formatter<'_, Query>) -> std::fmt::Result {
            struct Wrapper<'a, 'b>(&'a mut Formatter<'b, Query>);

            impl<'a, 'b> Drop for Wrapper<'a, 'b> {
                fn drop(&mut self) {
                    let _overlap = String::from("12345");
                    self.0.write_raw("world").ok();
                    assert!(self.0.prefixes.is_empty());
                }
            }

            let wrapper = Wrapper(formatter);
            let temporary_string = String::from("hello");

            // `write_named_value` will push `temp_string` into a buffer and
            // call the formatter for `MyValue`, which panics. At the panic
            // point, `formatter` contains an (illegal) static reference to
            // `temp_string` in its `prefixes` stack. When unwinding occurs,
            // `Wrapper` will be dropped. `Wrapper` holds a reference to
            // `Formatter`, thus `Formatter` must be consistent at this point.
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                wrapper.0.write_named_value(&temporary_string, MyValue)
            }));

            Ok(())
        }
    }

    #[test]
    fn check_consistency() {
        let string = format!("{}", &MyDisplay as &dyn UriDisplay<Query>);
        assert_eq!(string, "world");
    }
}
