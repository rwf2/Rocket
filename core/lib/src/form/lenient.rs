use std::ops::{Deref, DerefMut};

use crate::form::prelude::*;
use crate::http::uri::fmt::{Query, FromUriParam};

/// A form guard for parsing form types leniently.
///
/// This type implements the [`FromForm`] trait and thus can be used as a
/// generic parameter to the [`Form`] data guard: `Form<Lenient<T>>`, where `T`
/// implements `FromForm`. Unlike using `Form` directly, this type uses a
/// _lenient_ parsing strategy.
///
/// # Lenient Parsing
///
/// A `Lenient<T>` will parse successfully from an incoming form even if the
/// form contains extra or missing fields. If fields are missing, the form field
/// type's default will be used, if there is one. Extra fields are ignored; only
/// the first is parsed and validated. This is the default strategy for
/// [`Form`].
///
/// # Usage
///
/// `Lenient<T>` implements [`FromForm`] as long as `T` implements `FromForm`.
/// As such, `Form<Lenient<T>>` is a data guard.
///
/// Note that `Form<T>` _already_ parses leniently, so a `Form<Lenient<T>>` is
/// redundant and equal to `Form<T>`. `Lenient`, however, can be used to make
/// otherwise strict parses lenient, for example, in `Option<Lenient<T>>`:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// use rocket::form::Lenient;
///
/// #[derive(FromForm)]
/// struct UserInput {
///     // Parses as `Some(false)` when `lenient_inner_option` isn't present.
///     // Without `Lenient`, this would otherwise parse as `None`.
///     lenient_inner_option: Option<Lenient<bool>>,
/// }
/// ```
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Lenient<T>(T);

impl<T> Lenient<T> {
    /// Consumes `self` and returns the inner value.
    ///
    /// Note that since `Lenient` implements [`Deref`] and [`DerefMut`] with
    /// target `T`, reading and writing an inner value can be accomplished
    /// transparently.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::form::{Form, Lenient};
    ///
    /// #[derive(FromForm)]
    /// struct MyForm {
    ///     field: String,
    /// }
    ///
    /// #[post("/submit", data = "<form>")]
    /// fn submit(form: Form<Lenient<MyForm>>) -> String {
    ///     // We can read or mutate a value transparently:
    ///     let field: &str = &form.field;
    ///
    ///     // To gain ownership, however, use `into_inner()`:
    ///     form.into_inner().into_inner().field
    /// }
    /// ```
    pub fn into_inner(self) -> T {
        self.0
    }
}

#[crate::async_trait]
impl<'v, T: FromForm<'v>> FromForm<'v> for Lenient<T> {
    type Context = T::Context;

    #[inline(always)]
    fn init(opts: Options) -> Self::Context {
        T::init(Options { strict: false, ..opts })
    }

    #[inline(always)]
    fn push_value(ctxt: &mut Self::Context, field: ValueField<'v>) {
        T::push_value(ctxt, field)
    }

    #[inline(always)]
    async fn push_data(ctxt: &mut Self::Context, field: DataField<'v, '_>) {
        T::push_data(ctxt, field).await
    }

    #[inline(always)]
    fn finalize(this: Self::Context) -> Result<'v, Self> {
        T::finalize(this).map(Self)
    }
}

impl<T> Deref for Lenient<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Lenient<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> From<T> for Lenient<T> {
    #[inline]
    fn from(val: T) -> Lenient<T> {
        Lenient(val)
    }
}

impl<'f, A, T: FromUriParam<Query, A> + FromForm<'f>> FromUriParam<Query, A> for Lenient<T> {
    type Target = T::Target;

    #[inline(always)]
    fn from_uri_param(param: A) -> Self::Target {
        T::from_uri_param(param)
    }
}
