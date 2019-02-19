//! Types and traits for request parsing and handling.

mod form;
mod from_request;
mod param;
mod query;
mod request;
mod state;

#[cfg(test)]
mod tests;

pub use self::form::{Form, FormItem, FormItems, LenientForm};
pub use self::form::{FormDataError, FormError, FormParseError};
pub use self::form::{FromForm, FromFormValue};
pub use self::from_request::{FromRequest, Outcome};
pub use self::param::{FromParam, FromSegments};
pub use self::query::{FromQuery, Query};
pub use self::request::Request;
pub use self::state::State;

#[doc(inline)]
pub use response::flash::FlashMessage;
