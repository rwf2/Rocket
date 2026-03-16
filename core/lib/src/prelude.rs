//! The Rocket prelude.
//! includes the most commonly used types and macros for rapid development.

#[doc(inline)]
pub use crate::http::CookieJar;

#[doc(inline)]
pub use crate::{get, post, put, delete, patch, head, options, routes, launch};

#[doc(inline)]
pub use crate::State;

#[doc(inline)]
pub use crate::http::Status;

#[doc(inline)]
pub use crate::outcome::Outcome;

#[doc(inline)]
pub use crate::request::{Request, FlashMessage};

#[doc(inline)]
pub use crate::response::{Responder, Redirect, Flash};

#[doc(inline)]
pub use crate::form::{FromForm, FromFormField, Form};

#[cfg(feature = "json")]
#[doc(inline)]
pub use crate::serde::json::Json;

#[cfg(feature = "msgpack")]
#[doc(inline)]
pub use crate::serde::msgpack::MsgPack;

#[cfg(feature = "uuid")]
#[doc(inline)]
pub use crate::serde::uuid::uuid;
