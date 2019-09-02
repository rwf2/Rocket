//! Types and traits for request parsing and handling.

mod request;
mod param;
mod form;
mod from_request;
mod state;
mod query;

#[cfg(test)]
mod tests;

#[doc(hidden)] pub use rocket_codegen::{FromForm, FromFormValue};

pub use request::Request;
pub use from_request::{FromRequest, Outcome};
pub use param::{FromParam, FromSegments};
pub use form::{FromForm, FromFormValue};
pub use form::{Form, LenientForm, FormItems, FormItem};
pub use form::{FormError, FormParseError, FormDataError};
pub use self::state::State;
pub use query::{Query, FromQuery};

#[doc(inline)]
pub use crate::response::flash::FlashMessage;
