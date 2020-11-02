use std::ops::{Deref, DerefMut};

use crate::request::{Request, form::{Form, FormDataError, FromForm}};
use crate::data::{Data, Transformed, FromTransformedData, TransformFuture, FromDataFuture};
use crate::http::uri::{Query, FromUriParam};

/// A data guard for parsing [`FromForm`] types leniently.
///
/// This type implements the [`FromTransformedData`] trait, and like [`Form`], provides a
/// generic means to parse arbitrary structures from incoming form data. Unlike
/// `Form`, this type uses a _lenient_ parsing strategy: forms that contains a
/// superset of the expected fields (i.e, extra fields) will parse successfully.
///
/// # Leniency
///
/// A `LenientForm<T>` will parse successfully from an incoming form if the form
/// contains a superset of the fields in `T`. Said another way, a
/// `LenientForm<T>` automatically discards extra fields without error. For
/// instance, if an incoming form contains the fields "a", "b", and "c" while
/// `T` only contains "a" and "c", the form _will_ parse as `LenientForm<T>`.
///
/// # Usage
///
/// The usage of a `LenientForm` type is equivalent to that of [`Form`], so we
/// defer details to its documentation.
///
/// `LenientForm` implements `FromTransformedData`, so it can be used directly as a target
/// of the `data = "<param>"` route parameter. For instance, if some structure
/// of type `T` implements the `FromForm` trait, an incoming form can be
/// automatically parsed into the `T` structure with the following route and
/// handler:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// use rocket::request::LenientForm;
///
/// #[derive(FromForm)]
/// struct UserInput {
///     value: String
/// }
///
/// #[post("/submit", data = "<user_input>")]
/// fn submit_task(user_input: LenientForm<UserInput>) -> String {
///     format!("Your value: {}", user_input.value)
/// }
/// # fn main() {  }
/// ```
///
/// ## Incoming Data Limits
///
/// A `LenientForm` obeys the same data limits as a `Form` and defaults to
/// 32KiB. The limit can be increased by setting the `limits.forms`
/// configuration parameter. For instance, to increase the forms limit to 512KiB
/// for all environments, you may add the following to your `Rocket.toml`:
///
/// ```toml
/// [global.limits]
/// forms = 524288
/// ```
#[derive(Debug)]
pub struct LenientForm<T>(pub T);

impl<T> LenientForm<T> {
    /// Consumes `self` and returns the parsed value.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::request::LenientForm;
    ///
    /// #[derive(FromForm)]
    /// struct MyForm {
    ///     field: String,
    /// }
    ///
    /// #[post("/submit", data = "<form>")]
    /// fn submit(form: LenientForm<MyForm>) -> String {
    ///     form.into_inner().field
    /// }
    /// # fn main() { }
    #[inline(always)]
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for LenientForm<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> DerefMut for LenientForm<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<'r, T: FromForm<'r> + Send + 'r> FromTransformedData<'r> for LenientForm<T> {
    type Error = FormDataError<'r, T::Error>;
    type Owned = String;
    type Borrowed = str;

    fn transform(r: &'r Request<'_>, d: Data) -> TransformFuture<'r, Self::Owned, Self::Error> {
        <Form<T>>::transform(r, d)
    }

    fn from_data(_: &'r Request<'_>, o: Transformed<'r, Self>) -> FromDataFuture<'r, Self, Self::Error> {
        Box::pin(futures::future::ready(o.borrowed().and_then(|form| {
            <Form<T>>::from_data(form, false).map(LenientForm)
        })))
    }
}

impl<'r, A, T: FromUriParam<Query, A> + FromForm<'r>> FromUriParam<Query, A> for LenientForm<T> {
    type Target = T::Target;

    #[inline(always)]
    fn from_uri_param(param: A) -> Self::Target {
        T::from_uri_param(param)
    }
}
