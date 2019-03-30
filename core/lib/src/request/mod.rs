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

pub use self::request::Request;
pub use self::from_request::{FromRequest, Outcome};
pub use self::param::{FromParam, FromSegments};
pub use self::form::{FromForm, FromFormValue};
pub use self::form::{Form, LenientForm, FormItems, FormItem};
pub use self::form::{FormError, FormParseError, FormDataError};
pub use self::state::State;
pub use self::query::{Query, FromQuery};

#[doc(inline)]
pub use response::flash::FlashMessage;
