//! Types and traits for form processing.

mod error;
mod form;
mod form_items;
mod from_form;
mod from_form_value;
mod lenient;

pub use self::error::{FormDataError, FormError, FormParseError};
pub use self::form::Form;
pub use self::form_items::{FormItem, FormItems};
pub use self::from_form::FromForm;
pub use self::from_form_value::FromFormValue;
pub use self::lenient::LenientForm;
