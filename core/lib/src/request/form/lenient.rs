use std::fmt::{self, Debug};

use request::Request;
use request::form::{Form, FromForm};
use data::{self, Data, FromData};

/// A `FromData` type for parsing `FromForm` types leniently.
///
/// This type implements the `FromData` trait, and like
/// [`Form`](/rocket/request/struct.Form.html), provides a generic means to
/// parse arbitrary structures from incoming form data. Unlike `Form`, this type
/// uses a _lenient_ parsing strategy: forms that contains a superset of the
/// expected fields (i.e, extra fields) will parse successfully.
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
/// The usage of a `LenientForm` type is equivalent to that of
/// [`Form`](/rocket/request/struct.Form.html), so we defer details to its
/// documentation. We provide shallow information here.
///
/// `LenientForm` implements `FromData`, so it can be used directly as a target
/// of the `data = "<param>"` route parameter. For instance, if some structure
/// of type `T` implements the `FromForm` trait, an incoming form can be
/// automatically parsed into the `T` structure with the following route and
/// handler:
///
/// ```rust,ignore
/// #[post("/form_submit", data = "<param>")]
/// fn submit(form: LenientForm<T>) ... { ... }
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
pub struct LenientForm<'f, T: FromForm<'f> + 'f>(Form<'f, T>);

impl<'f, T: FromForm<'f> + 'f> LenientForm<'f, T> {
    /// Immutably borrow the parsed type.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #![feature(plugin, decl_macro)]
    /// # #![plugin(rocket_codegen)]
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
    ///     format!("Form field is: {}", form.get().field)
    /// }
    /// #
    /// # fn main() { }
    /// ```
    #[inline(always)]
    pub fn get(&'f self) -> &T {
        self.0.get()
    }

    /// Returns the raw form string that was used to parse the encapsulated
    /// object.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #![feature(plugin, decl_macro)]
    /// # #![plugin(rocket_codegen)]
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
    ///     format!("Raw form string is: {}", form.raw_form_string())
    /// }
    /// #
    /// # fn main() { }
    #[inline(always)]
    pub fn raw_form_string(&'f self) -> &str {
        self.0.raw_form_string()
    }
}

impl<'f, T: FromForm<'f> + 'static> LenientForm<'f, T> {
    /// Consumes `self` and returns the parsed value. For safety reasons, this
    /// method may only be called when the parsed value contains no
    /// non-`'static` references.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #![feature(plugin, decl_macro)]
    /// # #![plugin(rocket_codegen)]
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
    /// #
    /// # fn main() { }
    #[inline(always)]
    pub fn into_inner(self) -> T {
        self.0.into_inner()
    }
}

impl<'f, T: FromForm<'f> + Debug + 'f> Debug for LenientForm<'f, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'f, T: FromForm<'f>> FromData for LenientForm<'f, T> where T::Error: Debug {
    /// The raw form string, if it was able to be retrieved from the request.
    type Error = Option<String>;

    /// Parses a `LenientForm` from incoming form data.
    ///
    /// If the content type of the request data is not
    /// `application/x-www-form-urlencoded`, `Forward`s the request. If the form
    /// data cannot be parsed into a `T`, a `Failure` with status code
    /// `UnprocessableEntity` is returned. If the form string is malformed, a
    /// `Failure` with status code `BadRequest` is returned. Finally, if reading
    /// the incoming stream fails, returns a `Failure` with status code
    /// `InternalServerError`. In all failure cases, the raw form string is
    /// returned if it was able to be retrieved from the incoming stream.
    ///
    /// All relevant warnings and errors are written to the console in Rocket
    /// logging format.
    #[inline]
    fn from_data(request: &Request, data: Data) -> data::Outcome<Self, Self::Error> {
        super::from_data(request, data, false).map(LenientForm)
    }
}
