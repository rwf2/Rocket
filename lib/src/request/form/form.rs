use std::marker::PhantomData;
use std::fmt::{self, Debug};

use request::Request;
use data::{self, Data, FromData};
use request::form::{FromForm, FormItems};

/// A `FromData` type for parsing `FromForm` types strictly.
///
/// This type implements the `FromData` trait. It provides a generic means to
/// parse arbitrary structures from incoming form data.
///
/// # Strictness
///
/// A `Form<T>` will parse successfully from an incoming form only if the form
/// contains the exact set of fields in `T`. Said another way, a `Form<T>` will
/// error on missing and/or extra fields. For instance, if an incoming form
/// contains the fields "a", "b", and "c" while `T` only contains "a" and "c",
/// the form _will not_ parse as `Form<T>`. If you would like to admit extra
/// fields without error, see
/// [`LenientForm`](/rocket/request/struct.LenientForm.html).
///
/// # Usage
///
/// This type can be used with any type that implements the `FromForm` trait.
/// The trait can be automatically derived; see the
/// [FromForm](trait.FromForm.html) documentation for more information on
/// deriving or implementing the trait.
///
/// Because `Form` implements `FromData`, it can be used directly as a target of
/// the `data = "<param>"` route parameter. For instance, if some structure of
/// type `T` implements the `FromForm` trait, an incoming form can be
/// automatically parsed into the `T` structure with the following route and
/// handler:
///
/// ```rust,ignore
/// #[post("/form_submit", data = "<param>")]
/// fn submit(form: Form<T>) ... { ... }
/// ```
///
/// To preserve memory safety, if the underlying structure type contains
/// references into form data, the type can only be borrowed via the
/// [get](#method.get) or [get_mut](#method.get_mut) methods. Otherwise, the
/// parsed structure can be retrieved with the [into_inner](#method.into_inner)
/// method.
///
/// ## With References
///
/// The simplest data structure with a reference into form data looks like this:
///
/// ```rust
/// # #![feature(plugin, custom_derive)]
/// # #![allow(deprecated, dead_code, unused_attributes)]
/// # #![plugin(rocket_codegen)]
/// # extern crate rocket;
/// # use rocket::http::RawStr;
/// #[derive(FromForm)]
/// struct UserInput<'f> {
///     value: &'f RawStr
/// }
/// # fn main() {  }
/// ```
///
/// This corresponds to a form with a single field named `value` that should be
/// a string. A handler for this type can be written as:
///
/// ```rust
/// # #![feature(plugin, custom_derive)]
/// # #![allow(deprecated, unused_attributes)]
/// # #![plugin(rocket_codegen)]
/// # extern crate rocket;
/// # use rocket::request::Form;
/// # use rocket::http::RawStr;
/// # #[derive(FromForm)]
/// # struct UserInput<'f> {
/// #     value: &'f RawStr
/// # }
/// #[post("/submit", data = "<user_input>")]
/// fn submit_task<'r>(user_input: Form<'r, UserInput<'r>>) -> String {
///     format!("Your value: {}", user_input.get().value)
/// }
/// # fn main() {  }
/// ```
///
/// Note that the `` `r`` lifetime is used _twice_ in the handler's signature:
/// this is necessary to tie the lifetime of the structure to the lifetime of
/// the request data.
///
/// ## Without References
///
/// The owned analog of the `UserInput` type above is:
///
/// ```rust
/// # #![feature(plugin, custom_derive)]
/// # #![allow(deprecated, dead_code, unused_attributes)]
/// # #![plugin(rocket_codegen)]
/// # extern crate rocket;
/// #[derive(FromForm)]
/// struct OwnedUserInput {
///     value: String
/// }
/// # fn main() {  }
/// ```
///
/// The handler is written similarly:
///
/// ```rust
/// # #![feature(plugin, custom_derive)]
/// # #![allow(deprecated, unused_attributes)]
/// # #![plugin(rocket_codegen)]
/// # extern crate rocket;
/// # use rocket::request::Form;
/// # #[derive(FromForm)]
/// # struct OwnedUserInput {
/// #     value: String
/// # }
/// #[post("/submit", data = "<user_input>")]
/// fn submit_task(user_input: Form<OwnedUserInput>) -> String {
///     let input: OwnedUserInput = user_input.into_inner();
///     format!("Your value: {}", input.value)
/// }
/// # fn main() {  }
/// ```
///
/// Note that no lifetime annotations are required: Rust is able to infer the
/// lifetime as `` `static``. Because the lifetime is `` `static``, the
/// `into_inner` method can be used to directly retrieve the parsed value.
///
/// ## Performance and Correctness Considerations
///
/// Whether you should use a `&RawStr` or `String` in your `FromForm` type
/// depends on your use case. The primary question to answer is: _Can the input
/// contain characters that must be URL encoded?_ Note that this includes
/// commmon characters such as spaces. If so, then you must use `String`, whose
/// `FromFormValue` implementation automatically URL decodes strings. Because
/// the `&RawStr` references will refer directly to the underlying form data,
/// they will be raw and URL encoded.
///
/// If your string values will not contain URL encoded characters, using
/// `&RawStr` will result in fewer allocation and is thus preferred.
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
pub struct Form<'f, T: FromForm<'f> + 'f> {
    object: T,
    form_string: String,
    _phantom: PhantomData<&'f T>,
}

pub enum FormResult<T, E> {
    Ok(T),
    Err(String, E),
    Invalid(String)
}

impl<'f, T: FromForm<'f> + 'f> Form<'f, T> {
    /// Immutably borrow the parsed type.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #![feature(plugin, custom_derive)]
    /// # #![plugin(rocket_codegen)]
    /// # extern crate rocket;
    /// use rocket::request::Form;
    ///
    /// #[derive(FromForm)]
    /// struct MyForm {
    ///     field: String,
    /// }
    ///
    /// #[post("/submit", data = "<form>")]
    /// fn submit(form: Form<MyForm>) -> String {
    ///     format!("Form field is: {}", form.get().field)
    /// }
    /// #
    /// # fn main() { }
    /// ```
    #[inline(always)]
    pub fn get(&'f self) -> &T {
        &self.object
    }

    /// Returns the raw form string that was used to parse the encapsulated
    /// object.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #![feature(plugin, custom_derive)]
    /// # #![plugin(rocket_codegen)]
    /// # extern crate rocket;
    /// use rocket::request::Form;
    ///
    /// #[derive(FromForm)]
    /// struct MyForm {
    ///     field: String,
    /// }
    ///
    /// #[post("/submit", data = "<form>")]
    /// fn submit(form: Form<MyForm>) -> String {
    ///     format!("Raw form string is: {}", form.raw_form_string())
    /// }
    /// #
    /// # fn main() { }
    #[inline(always)]
    pub fn raw_form_string(&'f self) -> &str {
        &self.form_string
    }

    // Alright, so here's what's going on here. We'd like to have form
    // objects have pointers directly to the form string. This means that
    // the form string has to live at least as long as the form object. So,
    // to enforce this, we store the form_string along with the form object.
    //
    // So far so good. Now, this means that the form_string can never be
    // deallocated while the object is alive. That implies that the
    // `form_string` value should never be moved away. We can enforce that
    // easily by 1) not making `form_string` public, and 2) not exposing any
    // `&mut self` methods that could modify `form_string`.
    //
    // Okay, we do all of these things. Now, we still need to give a
    // lifetime to `FromForm`. Which one do we choose? The danger is that
    // references inside `object` may be copied out, and we have to ensure
    // that they don't outlive this structure. So we would really like
    // something like `self` and then to transmute to that. But this doesn't
    // exist. So we do the next best: we use the first lifetime supplied by the
    // caller via `get()` and contrain everything to that lifetime. This is, in
    // reality a little coarser than necessary, but the user can simply move the
    // call to right after the creation of a Form object to get the same effect.
    pub(crate) fn new(string: String, strict: bool) -> FormResult<Self, T::Error> {
        let long_lived_string: &'f str = unsafe {
            ::std::mem::transmute(string.as_str())
        };

        let mut items = FormItems::from(long_lived_string);
        let result = T::from_form(items.by_ref(), strict);
        if !items.exhaust() {
            return FormResult::Invalid(string);
        }

        match result {
            Ok(obj) => FormResult::Ok(Form {
                form_string: string,
                object: obj,
                _phantom: PhantomData
            }),
            Err(e) => FormResult::Err(string, e)
        }
    }
}

impl<'f, T: FromForm<'f> + 'static> Form<'f, T> {
    /// Consumes `self` and returns the parsed value. For safety reasons, this
    /// method may only be called when the parsed value contains no
    /// non-`'static` references.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #![feature(plugin, custom_derive)]
    /// # #![plugin(rocket_codegen)]
    /// # extern crate rocket;
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
    /// #
    /// # fn main() { }
    #[inline(always)]
    pub fn into_inner(self) -> T {
        self.object
    }
}

impl<'f, T: FromForm<'f> + Debug + 'f> Debug for Form<'f, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?} from form string: {:?}", self.object, self.form_string)
    }
}

impl<'f, T: FromForm<'f>> FromData for Form<'f, T> where T::Error: Debug {
    /// The raw form string, if it was able to be retrieved from the request.
    type Error = Option<String>;

    /// Parses a `Form` from incoming form data.
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
        super::from_data(request, data, true)
    }
}
