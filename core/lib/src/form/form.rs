use std::ops::{Deref, DerefMut};

use crate::Request;
use crate::outcome::try_outcome;
use crate::data::{Data, FromData, Outcome};
use crate::http::{RawStr, ext::IntoOwned};
use crate::form::parser::{Parser, RawStrParser, Buffer};
use crate::form::prelude::*;

/// A data guard for [`FromForm`] types.
///
/// This type implements the [`FromData`] trait. It provides a generic means to
/// parse arbitrary structures from incoming form data.
///
/// See the [forms guide](https://rocket.rs/v0.5-rc/guide/requests#forms) for
/// general form support documentation.
///
/// # Leniency
///
/// A `Form<T>` will parse successfully from an incoming form if the form
/// contains a superset of the fields in `T`. Said another way, a `Form<T>`
/// automatically discards extra fields without error. For instance, if an
/// incoming form contains the fields "a", "b", and "c" while `T` only contains
/// "a" and "c", the form _will_ parse as `Form<T>`. To parse strictly, use the
/// [`Strict`](crate::form::Strict) form guard.
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
/// # #[macro_use] extern crate rocket;
/// use rocket::form::Form;
/// use rocket::http::RawStr;
///
/// #[derive(FromForm)]
/// struct UserInput<'r> {
///     value: &'r str
/// }
///
/// #[post("/submit", data = "<user_input>")]
/// fn submit_task(user_input: Form<UserInput<'_>>) -> String {
///     format!("Your value: {}", user_input.value)
/// }
/// ```
///
/// A type of `Form<T>` automatically dereferences into an `&T` or `&mut T`,
/// though you can also transform a `Form<T>` into a `T` by calling
/// [`into_inner()`](Form::into_inner()). Thanks to automatic dereferencing, you
/// can access fields of `T` transparently through a `Form<T>`, as seen above
/// with `user_input.value`.
///
/// ## Data Limits
///
/// The total amount of data accepted by the `Form` data guard is limited by the
/// following limits:
///
/// | Limit Name  | Default | Description                        |
/// |-------------|---------|------------------------------------|
/// | `form`      | 32KiB   | total limit for url-encoded forms  |
/// | `data-form` | 2MiB    | total limit for multipart forms    |
/// | `*`         | N/A     | each field type has its own limits |
///
/// As noted above, each form field type (a form guard) typically imposes its
/// own limits. For example, the `&str` form guard imposes a data limit of
/// `string` when multipart data is streamed.
///
/// ### URL-Encoded Forms
///
/// The `form` limit specifies the data limit for an entire url-encoded form
/// data. It defaults to 32KiB. URL-encoded form data is percent-decoded, stored
/// in-memory, and parsed into [`ValueField`]s. If the incoming data exceeds
/// this limit, the `Form` data guard fails without attempting to parse fields
/// with a `413: Payload Too Large` error.
///
/// ### Multipart Forms
///
/// The `data-form` limit specifies the data limit for an entire multipart form
/// data stream. It defaults to 2MiB. Multipart data is streamed, and form
/// fields are processed into [`DataField`]s or [`ValueField`]s as they arrive.
/// If the commulative data received while streaming exceeds the limit, parsing
/// is aborted, an error is created and pushed via [`FromForm::push_error()`],
/// and the form is finalized.
///
/// ### Individual Fields
///
/// Individual fields _may_ have data limits as well. The type of the field
/// determines whether there is a data limit. For instance, the `&str` type
/// imposes the `string` data limit. Consult the type's documentation or
/// [`FromFormField`] for details.
///
/// ### Changing Limits
///
/// To change data limits, set the `limits.form` and/or `limits.data-form`
/// configuration parameters. For instance, to increase the URL-encoded forms
/// limit to 128KiB for all environments, you might add the following to your
/// `Rocket.toml`:
///
/// ```toml
/// [global.limits]
/// form = 128KiB
/// ```
///
/// See the [`Limits`](crate::data::Limits) and [`config`](crate::config) docs
/// for more.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Form<T>(T);

impl<T> Form<T> {
    /// Consumes `self` and returns the inner value.
    ///
    /// Note that since `Form` implements [`Deref`] and [`DerefMut`] with
    /// target `T`, reading and writing an inner value can be accomplished
    /// transparently.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::form::Form;
    ///
    /// #[derive(FromForm)]
    /// struct MyForm {
    ///     field: String,
    /// }
    ///
    /// #[post("/submit", data = "<form>")]
    /// fn submit(form: Form<MyForm>) -> String {
    ///     // We can read or mutate a value transparently:
    ///     let field: &str = &form.field;
    ///
    ///     // To gain ownership, however, use `into_inner()`:
    ///     form.into_inner().field
    /// }
    /// ```
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> From<T> for Form<T> {
    #[inline]
    fn from(val: T) -> Form<T> {
        Form(val)
    }
}

impl<'r, T: FromForm<'r>> Form<T> {
    /// Leniently parses a `T` from a **percent-decoded**
    /// `x-www-form-urlencoded` form string. Specifically, this method
    /// implements [§5.1 of the WHATWG URL Living Standard] with the exception
    /// of steps 3.4 and 3.5, which are assumed to already be reflected in
    /// `string`, and then parses the fields as `T`.
    ///
    /// [§5.1 of the WHATWG URL Living Standard]: https://url.spec.whatwg.org/#application/x-www-form-urlencoded
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::form::{Form, FromForm};
    ///
    /// #[derive(FromForm)]
    /// struct Pet<'r> {
    ///     name: &'r str,
    ///     wags: bool,
    /// }
    ///
    /// let string = "name=Benson Wagger!&wags=true";
    /// let pet: Pet<'_> = Form::parse(string).unwrap();
    /// assert_eq!(pet.name, "Benson Wagger!");
    /// assert_eq!(pet.wags, true);
    /// ```
    #[inline]
    pub fn parse(string: &'r str) -> Result<'r, T> {
        // WHATWG URL Living Standard 5.1 steps 1, 2, 3.1 - 3.3.
        Self::parse_iter(Form::values(string))
    }

    /// Leniently parses a `T` from the **percent-decoded** `fields`.
    /// Specifically, this method implements [§5.1 of the WHATWG URL Living
    /// Standard] with the exception of step 3.
    ///
    /// [§5.1 of the WHATWG URL Living Standard]: https://url.spec.whatwg.org/#application/x-www-form-urlencoded
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::form::{Form, FromForm, ValueField};
    ///
    /// #[derive(FromForm)]
    /// struct Pet<'r> {
    ///     name: &'r str,
    ///     wags: bool,
    /// }
    ///
    /// let fields = vec![
    ///     ValueField::parse("name=Bob, the cat. :)"),
    ///     ValueField::parse("wags=no"),
    /// ];
    ///
    /// let pet: Pet<'_> = Form::parse_iter(fields).unwrap();
    /// assert_eq!(pet.name, "Bob, the cat. :)");
    /// assert_eq!(pet.wags, false);
    /// ```
    pub fn parse_iter<I>(fields: I) -> Result<'r, T>
        where I: IntoIterator<Item = ValueField<'r>>
    {
        // WHATWG URL Living Standard 5.1 steps 1, 2, 3.1 - 3.3.
        let mut ctxt = T::init(Options::Lenient);
        fields.into_iter().for_each(|f| T::push_value(&mut ctxt, f));
        T::finalize(ctxt)
    }
}

impl<T: for<'a> FromForm<'a> + 'static> Form<T> {
    /// Leniently parses a `T` from a raw, `x-www-form-urlencoded` form string.
    /// Specifically, this method implements [§5.1 of the WHATWG URL Living
    /// Standard]. Because percent-decoding might modify the input string, the
    /// output type `T` must be `'static`.
    ///
    /// [§5.1 of the WHATWG URL Living Standard]:https://url.spec.whatwg.org/#application/x-www-form-urlencoded
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::http::RawStr;
    /// use rocket::form::{Form, FromForm};
    ///
    /// #[derive(FromForm)]
    /// struct Pet {
    ///     name: String,
    ///     wags: bool,
    /// }
    ///
    /// let string = RawStr::new("name=Benson+Wagger%21&wags=true");
    /// let pet: Pet = Form::parse_encoded(string).unwrap();
    /// assert_eq!(pet.name, "Benson Wagger!");
    /// assert_eq!(pet.wags, true);
    /// ```
    pub fn parse_encoded(string: &RawStr) -> Result<'static, T> {
        let buffer = Buffer::new();
        let mut ctxt = T::init(Options::Lenient);
        for field in RawStrParser::new(&buffer, string) {
            T::push_value(&mut ctxt, field)
        }

        T::finalize(ctxt).map_err(|e| e.into_owned())
    }
}

impl Form<()> {
    /// Returns an iterator of fields parsed from a `x-www-form-urlencoded` form
    /// string. Specifically, this method implements steps 1, 2, and 3.1 - 3.3
    /// of [§5.1 of the WHATWG URL Living Standard]. Fields in the returned
    /// iterator _are not_ percent-decoded.
    ///
    /// [§5.1 of the WHATWG URL Living Standard]:https://url.spec.whatwg.org/#application/x-www-form-urlencoded
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::form::{Form, ValueField};
    ///
    /// let string = "name=Bobby Brown&&&email=me@rocket.rs";
    /// let mut values = Form::values(string);
    /// assert_eq!(values.next().unwrap(), ValueField::parse("name=Bobby Brown"));
    /// assert_eq!(values.next().unwrap(), ValueField::parse("email=me@rocket.rs"));
    /// assert!(values.next().is_none());
    /// ```
    pub fn values(string: &str) -> impl Iterator<Item = ValueField<'_>> {
        // WHATWG URL Living Standard 5.1 steps 1, 2, 3.1 - 3.3.
        string.split('&')
            .filter(|s| !s.is_empty())
            .map(ValueField::parse)
    }
}

impl<T> Deref for Form<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Form<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[crate::async_trait]
impl<'r, T: FromForm<'r>> FromData<'r> for Form<T> {
    type Error = Errors<'r>;

    async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> Outcome<'r, Self> {
        use either::Either;

        let mut parser = try_outcome!(Parser::new(req, data).await);
        let mut context = T::init(Options::Lenient);
        while let Some(field) = parser.next().await {
            match field {
                Ok(Either::Left(value)) => T::push_value(&mut context, value),
                Ok(Either::Right(data)) => T::push_data(&mut context, data).await,
                Err(e) => T::push_error(&mut context, e),
            }
        }

        match T::finalize(context) {
            Ok(value) => Outcome::Success(Form(value)),
            Err(e) => Outcome::Failure((e.status(), e)),
        }
    }
}
