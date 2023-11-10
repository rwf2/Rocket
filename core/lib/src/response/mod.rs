//! Types and traits to build and send responses.
//!
//! The return type of a Rocket handler can be any type that implements the
//! [`Responder`] trait, which means that the type knows how to generate a
//! [`Response`]. Among other things, this module contains several such types.
//!
//! # Composing
//!
//! Many of the built-in `Responder` types _chain_ responses: they take in
//! another `Responder` and add, remove, or change information in the response.
//! In other words, many `Responder` types are built to compose well. As a
//! result, you'll often have types of the form `A<B<C>>` consisting of three
//! `Responder`s `A`, `B`, and `C`. This is normal and encouraged as the type
//! names typically illustrate the intended response.

mod responder;
mod redirect;
mod response;
mod debug;
mod body;

pub(crate) mod flash;

pub mod content;
pub mod status;
pub mod stream;

#[doc(hidden)]
pub use rocket_codegen::Responder;

pub use self::response::{Response, Builder};
pub use self::body::Body;
pub use self::responder::Responder;
pub use self::redirect::Redirect;
pub use self::flash::Flash;
pub use self::debug::Debug;

/// Type alias for the `Result` of a [`Responder::respond_to()`] call.
pub type Result<'r> = std::result::Result<Response<'r>, crate::http::Status>;
