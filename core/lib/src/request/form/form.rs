use std::ops::Deref;

use crate::outcome::Outcome::*;
use crate::request::{Request, form::{FromForm, FormItems, FormDataError}};
use crate::data::{Outcome, Transform, Transformed, Data, FromData};
use crate::http::{Status, uri::{Query, FromUriParam}};

/// A data guard for parsing [`FromForm`] types strictly.
///
/// This type implements the [`FromData`] trait. It provides a generic means to
/// parse arbitrary structures from incoming form data.
///
/// # Strictness
///
/// A `Form<T>` will parse successfully from an incoming form only if the form
/// contains the exact set of fields in `T`. Said another way, a `Form<T>` will
/// error on missing and/or extra fields. For instance, if an incoming form
/// contains the fields "a", "b", and "c" while `T` only contains "a" and "c",
/// the form _will not_ parse as `Form<T>`. If you would like to admit extra
/// fields without error, see [`LenientForm`](crate::request::LenientForm).
///
/// # Usage
///
/// This type can be used with any type that implements the `FromForm` trait.
/// The trait can be automatically derived; see the [`FromForm`] documentation
/// for more information on deriving or implementing the trait.
///
/// Because `Form` implements `FromData`, it can be used directly as a target of
/// the `data = "<param>"` route parameter as long as its generic type
/// implements the `FromForm` trait:
///
/// ```rust
/// # #![feature(proc_macro_hygiene, decl_macro)]
/// # #[macro_use] extern crate rocket;
/// use rocket::request::Form;
/// use rocket::http::RawStr;
///
/// #[derive(FromForm)]
/// struct UserInput<'f> {
///     // The raw, undecoded value. You _probably_ want `String` instead.
///     value: &'f RawStr
/// }
///
/// #[post("/submit", data = "<user_input>")]
/// fn submit_task(user_input: Form<UserInput>) -> String {
///     format!("Your value: {}", user_input.value)
/// }
/// # fn main() {  }
/// ```
///
/// A type of `Form<T>` automatically dereferences into an `&T`, though you can
/// also transform a `Form<T>` into a `T` by calling
/// [`into_inner()`](Form::into_inner()). Thanks to automatic dereferencing, you
/// can access fields of `T` transparently through a `Form<T>`, as seen above
/// with `user_input.value`.
///
/// For posterity, the owned analog of the `UserInput` type above is:
///
/// ```rust
/// struct OwnedUserInput {
///     // The decoded value. You _probably_ want this.
///     value: String
/// }
/// ```
///
/// A handler that handles a form of this type can similarly by written:
///
/// ```rust
/// # #![feature(proc_macro_hygiene, decl_macro)]
/// # #![allow(deprecated, unused_attributes)]
/// # #[macro_use] extern crate rocket;
/// # use rocket::request::Form;
/// # #[derive(FromForm)]
/// # struct OwnedUserInput {
/// #     value: String
/// # }
/// #[post("/submit", data = "<user_input>")]
/// fn submit_task(user_input: Form<OwnedUserInput>) -> String {
///     format!("Your value: {}", user_input.value)
/// }
/// # fn main() {  }
/// ```
///
/// Note that no lifetime annotations are required in either case.
///
/// ## `&RawStr` vs. `String`
///
/// Whether you should use a `&RawStr` or `String` in your `FromForm` type
/// depends on your use case. The primary question to answer is: _Can the input
/// contain characters that must be URL encoded?_ Note that this includes common
/// characters such as spaces. If so, then you must use `String`, whose
/// [`FromFormValue`](crate::request::FromFormValue) implementation automatically URL
/// decodes the value. Because the `&RawStr` references will refer directly to
/// the underlying form data, they will be raw and URL encoded.
///
/// If it is known that string values will not contain URL encoded characters,
/// or you wish to handle decoding and validation yourself, using `&RawStr` will
/// result in fewer allocation and is thus preferred.
///
/// ## Incoming Data Limits
///
/// The default size limit for incoming form data is 32KiB. Setting a limit
/// protects your application from denial of service (DOS) attacks and from
/// resource exhaustion through high memory consumption. The limit can be
/// increased by setting the `limits.forms` configuration parameter. For
/// instance, to increase the forms limit to 512KiB for all environments, you
/// may add the following to your `Rocket.toml`:
///
/// ```toml
/// [global.limits]
/// forms = 524288
/// ```
#[derive(Debug)]
pub struct Form<T>(pub T);

impl<T> Form<T> {
    /// Consumes `self` and returns the parsed value.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #![feature(proc_macro_hygiene, decl_macro)]
    /// # #[macro_use] extern crate rocket;
    /// use rocket::request::Form;
    ///
    /// #[derive(FromForm)]
    /// struct MyForm {
    ///     field: String,
    /// }
    ///
    /// #[post("/submit", data = "<form>")]
    /// fn submit(form: Form<MyForm>) -> String {
    ///     form.into_inner().field
    /// }
    /// # fn main() { }
    #[inline(always)]
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for Form<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<'f, T: FromForm<'f>> Form<T> {
    crate fn from_data(
        form_str: &'f str,
        strict: bool
    ) -> Outcome<T, FormDataError<'f, T::Error>> {
        use self::FormDataError::*;

        let mut items = FormItems::from(form_str);
        let result = T::from_form(&mut items, strict);
        if !items.exhaust() {
            error_!("The request's form string was malformed.");
            return Failure((Status::BadRequest, Malformed(form_str)));
        }

        match result {
            Ok(v) => Success(v),
            Err(e) => {
                error_!("The incoming form failed to parse.");
                Failure((Status::UnprocessableEntity, Parse(e, form_str)))
            }
        }
    }
}

/// Parses a `Form` from incoming form data.
///
/// If the content type of the request data is not
/// `application/x-www-form-urlencoded`, `Forward`s the request. If the form
/// data cannot be parsed into a `T`, a `Failure` with status code
/// `UnprocessableEntity` is returned. If the form string is malformed, a
/// `Failure` with status code `BadRequest` is returned. Finally, if reading the
/// incoming stream fails, returns a `Failure` with status code
/// `InternalServerError`. In all failure cases, the raw form string is returned
/// if it was able to be retrieved from the incoming stream.
///
/// All relevant warnings and errors are written to the console in Rocket
/// logging format.
impl<'f, T: FromForm<'f>> FromData<'f> for Form<T> {
    type Error = FormDataError<'f, T::Error>;
    type Owned = String;
    type Borrowed = str;

    fn transform(
        request: &Request<'_>,
        data: Data
    ) -> Transform<Outcome<Self::Owned, Self::Error>> {
        use std::{cmp::min, io::Read};

        let outcome = 'o: {
            if !request.content_type().map_or(false, |ct| ct.is_form()) {
                warn_!("Form data does not have form content type.");
                break 'o Forward(data);
            }

            let limit = request.limits().forms;
            let mut stream = data.open().take(limit);
            let mut form_string = String::with_capacity(min(4096, limit) as usize);
            if let Err(e) = stream.read_to_string(&mut form_string) {
                break 'o Failure((Status::InternalServerError, FormDataError::Io(e)));
            }

            break 'o Success(form_string);
        };

        Transform::Borrowed(outcome)
    }

    fn from_data(_: &Request<'_>, o: Transformed<'f, Self>) -> Outcome<Self, Self::Error> {
        <Form<T>>::from_data(o.borrowed()?, true).map(Form)
    }
}

impl<'f, A, T: FromUriParam<Query, A> + FromForm<'f>> FromUriParam<Query, A> for Form<T> {
    type Target = T::Target;

    #[inline(always)]
    fn from_uri_param(param: A) -> Self::Target {
        T::from_uri_param(param)
    }
}
