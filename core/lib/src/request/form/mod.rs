//! Types and traits for form processing.

mod form_items;
mod from_form;
mod from_form_value;
mod lenient;
mod error;
mod form;

pub use form_items::{FormItems, FormItem};
pub use from_form::FromForm;
pub use from_form_value::FromFormValue;
pub use form::Form;
pub use lenient::LenientForm;
pub use error::{FormError, FormParseError, FormDataError};
