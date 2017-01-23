//! Types and traits for request parsing and handling.

mod request;
mod param;
mod form;
mod from_request;
mod state;

pub use self::request::Request;
pub use self::from_request::{FromRequest, Outcome};
pub use self::param::{FromParam, FromSegments};
pub use self::form::{Form, FromForm, FromFormValue, FormItems};
pub use self::state::State;

/// Type alias to retrieve flash messages from a request.
pub type FlashMessage = ::response::Flash<()>;
