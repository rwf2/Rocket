//! Support for using [`Error`](crate::anyhow::Error) from [`mod@anyhow`]
//! as a [`Responder`].

#[doc(inline)]
pub use anyhow::*;

use crate::http::Status;
use crate::request::Request;
use crate::response::{self, Responder};

/// Prints a warning with the error and forwards to the `500` error catcher.
impl<'r> Responder<'r, 'static> for anyhow::Error {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        warn_!("Error: {:#}", yansi::Paint::default(self));
        Err(Status::InternalServerError)
    }
}
