//! Types and traits to handle form processing.
//!
//! In general, you will deal with forms in Rocket via the `form` parameter in
//! routes:
//!
//! ```rust,ignore
//! #[post("/", form = <my_form>)]
//! fn form_submit(my_form: MyType) -> ...
//! ```
//!
//! Form parameter types must implement the [FromForm](trait.FromForm.html)
//! trait, which is auto-derivable. Automatically deriving `FromForm` for a
//! structure requires that all of its fields implement
//! [FromFormValue](trait.FormFormValue.html), which parses and validates form
//! fields. See the [codegen](/rocket_codegen/) documentation or the [forms
//! guide](/guide/forms) for more information on forms and on deriving
//! `FromForm`.

mod form_items;
mod from_form;
mod from_form_value;

pub use self::form_items::FormItems;
pub use self::from_form::FromForm;
pub use self::from_form_value::FromFormValue;

use std::marker::PhantomData;
use std::fmt::{self, Debug};
use std::io::Read;

use http::Status;
use request::Request;
use data::{self, Data, FromData};
use outcome::Outcome::*;

// TODO: This works and is safe, but the lifetime appears twice.
/// A `FromData` type for parsing `FromForm` types.
///
/// This type implements the `FromData` trait. It provides a generic means to
/// parse arbitrary structures from incoming form data.
///
/// # Usage
///
/// This type can be used with any type that implements the `FromForm` trait.
/// The trait can be automatically derived; see the
/// [FromForm](trait.FromForm.html) documentation for more information on
/// deriving or implementing the trait.
///
/// Because `Form` implement `FromData`, it can be used directly as a target of
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
/// Whether you should use a `str` or `String` in your `FromForm` type depends
/// on your use case. The primary question to answer is: _Can the input contain
/// characters that must be URL encoded?_ Note that this includes commmon
/// characters such as spaces. If so, then you must use `String`, whose
/// `FromFormValue` implementation deserializes the URL encoded string for you.
/// Because the `str` references will refer directly to the underlying form
/// data, they will be raw and URL encoded.
///
/// If your string values will not contain URL encoded characters, using `str`
/// will result in fewer allocation and is thus spreferred.
pub struct Form<'f, T: FromForm<'f> + 'f> {
    object: T,
    form_string: String,
    _phantom: PhantomData<&'f T>,
}

enum FormResult<T, E> {
    Ok(T),
    Err(String, E),
    Invalid(String)
}

#[cfg(test)]
impl<T, E> FormResult<T, E> {
    fn unwrap(self) -> T {
        match self {
            FormResult::Ok(val) => val,
            _ => panic!("Unwrapping non-Ok FormResult.")
        }
    }
}

impl<'f, T: FromForm<'f> + 'f> Form<'f, T> {
    /// Immutably borrow the parsed type.
    pub fn get(&'f self) -> &'f T {
        &self.object
    }

    /// Mutably borrow the parsed type.
    pub fn get_mut(&'f mut self) -> &'f mut T {
        &mut self.object
    }

    /// Returns the raw form string that was used to parse the encapsulated
    /// object.
    pub fn raw_form_string(&self) -> &str {
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
    fn new(form_string: String) -> FormResult<Self, T::Error> {
        let long_lived_string: &'f str = unsafe {
            ::std::mem::transmute(form_string.as_str())
        };

        let mut items = FormItems::from(long_lived_string);
        let result = T::from_form_items(items.by_ref());
        if !items.exhaust() {
            return FormResult::Invalid(form_string);
        }

        match result {
            Ok(obj) => FormResult::Ok(Form {
                form_string: form_string,
                object: obj,
                _phantom: PhantomData
            }),
            Err(e) => FormResult::Err(form_string, e)
        }
    }
}

impl<'f, T: FromForm<'f> + 'static> Form<'f, T> {
    /// Consume this object and move out the parsed object.
    pub fn into_inner(self) -> T {
        self.object
    }
}

impl<'f, T: FromForm<'f> + Debug + 'f> Debug for Form<'f, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?} from form string: {:?}", self.object, self.form_string)
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
impl<'f, T: FromForm<'f>> FromData for Form<'f, T> where T::Error: Debug {
    /// The raw form string, if it was able to be retrieved from the request.
    type Error = Option<String>;

    fn from_data(request: &Request, data: Data) -> data::Outcome<Self, Self::Error> {
        if !request.content_type().map_or(false, |ct| ct.is_form()) {
            warn_!("Form data does not have form content type.");
            return Forward(data);
        }

        let mut form_string = String::with_capacity(4096);
        let mut stream = data.open().take(32768); // TODO: Make this configurable?
        if let Err(e) = stream.read_to_string(&mut form_string) {
            error_!("IO Error: {:?}", e);
            Failure((Status::InternalServerError, None))
        } else {
            match Form::new(form_string) {
                FormResult::Ok(form) => Success(form),
                FormResult::Invalid(form_string) => {
                    error_!("The request's form string was malformed.");
                    Failure((Status::BadRequest, Some(form_string)))
                }
                FormResult::Err(form_string, e) => {
                    error_!("Failed to parse value from form: {:?}", e);
                    Failure((Status::UnprocessableEntity, Some(form_string)))
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::Form;
    use ::request::{FromForm, FormItems};

    struct Simple<'s> {
        value: &'s str
    }

    struct Other {
        value: String
    }

    impl<'s> FromForm<'s> for Simple<'s> {
        type Error = ();

        fn from_form_items(items: &mut FormItems<'s>) -> Result<Simple<'s>, ()> {
            Ok(Simple { value: items.inner_str() })
        }
    }

    impl<'s> FromForm<'s> for Other {
        type Error = ();

        fn from_form_items(items: &mut FormItems<'s>) -> Result<Other, ()> {
            Ok(Other { value: items.inner_str().to_string() })
        }
    }

    #[test]
    fn test_lifetime() {
        let form_string = "hello=world".to_string();
        let form: Form<Simple> = Form::new(form_string).unwrap();

        let string: &str = form.get().value;
        assert_eq!(string, "hello=world");
    }

    #[test]
    fn test_lifetime_2() {
        let form_string = "hello=world".to_string();
        let mut _y = "hi";
        let _form: Form<Simple> = Form::new(form_string).unwrap();
        // _y = form.get().value;

        // fn should_not_compile<'f>(form: Form<'f, &'f str>) -> &'f str {
        //     form.get()
        // }

        // fn should_not_compile_2<'f>(form: Form<'f, Simple<'f>>) -> &'f str {
        //     form.into_inner().value
        // }

        // assert_eq!(should_not_compile(form), "hello=world");
    }

    #[test]
    fn test_lifetime_3() {
        let form_string = "hello=world".to_string();
        let form: Form<Other> = Form::new(form_string).unwrap();

        // Not bad.
        fn should_compile(form: Form<Other>) -> String {
            form.into_inner().value
        }

        assert_eq!(should_compile(form), "hello=world".to_string());
    }

    #[test]
    fn test_lifetime_4() {
        let form_string = "hello=world".to_string();
        let form: Form<Simple> = Form::new(form_string).unwrap();

        fn should_compile<'f>(_form: Form<'f, Simple<'f>>) {  }

        should_compile(form)
        // assert_eq!(should_not_compile(form), "hello=world");
    }
}

