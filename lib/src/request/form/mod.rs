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
mod form;
mod lenient;

pub use self::form_items::FormItems;
pub use self::from_form::FromForm;
pub use self::from_form_value::FromFormValue;
pub use self::form::Form;
pub use self::lenient::LenientForm;

use std::cmp;
use std::io::Read;
use std::fmt::Debug;

use outcome::Outcome::*;
use request::Request;
use data::{self, Data};
use self::form::FormResult;
use http::Status;

fn from_data<'f, T>(request: &Request,
                    data: Data,
                    strict: bool
                   ) -> data::Outcome<Form<'f, T>, Option<String>>
    where T: FromForm<'f>, T::Error: Debug
{
    if !request.content_type().map_or(false, |ct| ct.is_form()) {
        warn_!("Form data does not have form content type.");
        return Forward(data);
    }

    let limit = request.limits().forms;
    let mut form_string = String::with_capacity(cmp::min(4096, limit) as usize);
    let mut stream = data.open().take(limit);
    if let Err(e) = stream.read_to_string(&mut form_string) {
        error_!("IO Error: {:?}", e);
        Failure((Status::InternalServerError, None))
    } else {
        match Form::new(form_string, strict) {
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

#[cfg(test)]
mod test {
    use super::{Form, FormResult};
    use ::request::{FromForm, FormItems};

    impl<T, E> FormResult<T, E> {
        fn unwrap(self) -> T {
            match self {
                FormResult::Ok(val) => val,
                _ => panic!("Unwrapping non-Ok FormResult.")
            }
        }
    }

    struct Simple<'s> {
        value: &'s str
    }

    struct Other {
        value: String
    }

    impl<'s> FromForm<'s> for Simple<'s> {
        type Error = ();

        fn from_form(items: &mut FormItems<'s>, _: bool) -> Result<Simple<'s>, ()> {
            Ok(Simple { value: items.inner_str() })
        }
    }

    impl<'s> FromForm<'s> for Other {
        type Error = ();

        fn from_form(items: &mut FormItems<'s>, _: bool) -> Result<Other, ()> {
            Ok(Other { value: items.inner_str().to_string() })
        }
    }

    #[test]
    fn test_lifetime() {
        let form_string = "hello=world".to_string();
        let form: Form<Simple> = Form::new(form_string, true).unwrap();

        let string: &str = form.get().value;
        assert_eq!(string, "hello=world");
    }

    #[test]
    fn test_lifetime_2() {
        let form_string = "hello=world".to_string();
        let mut _y = "hi";
        let _form: Form<Simple> = Form::new(form_string, true).unwrap();
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
        let form: Form<Other> = Form::new(form_string, true).unwrap();

        // Not bad.
        fn should_compile(form: Form<Other>) -> String {
            form.into_inner().value
        }

        assert_eq!(should_compile(form), "hello=world".to_string());
    }

    #[test]
    fn test_lifetime_4() {
        let form_string = "hello=world".to_string();
        let form: Form<Simple> = Form::new(form_string, true).unwrap();

        fn should_compile<'f>(_form: Form<'f, Simple<'f>>) {  }

        should_compile(form)
        // assert_eq!(should_not_compile(form), "hello=world");
    }
}

