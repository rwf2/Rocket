//! Types and traits to build and send responses.
//!
//! The return type of a Rocket handler can be any type that implements the
//! [Responder](trait.Responder.html) trait. Among other things, this module
//! contains several such types.
//!
//! # Composing
//!
//! Many of the built-in `Responder` types _chain_ responses: they take in
//! another `Responder` and add, remove, or change information in the response.
//! In other words, many `Responder` types are built to compose well. As a
//! result, you'll often have types of the form `A<B<C>>` consisting of three
//! `Responder`s `A`, `B`, and `C`. This is normal and encouraged as the type
//! names typically illustrate the intended response.
//!
//! # Contrib
//!
//! The [`contrib` crate](/rocket_contrib) contains several useful `Responder`s
//! including [`Template`](/rocket_contrib/struct.Template.html) and
//! [`Json`](/rocket_contrib/struct.Json.html).

mod responder;
mod redirect;
mod flash;
mod named_file;
mod stream;
mod response;
mod failure;

pub mod content;
pub mod status;

pub use self::response::{Response, ResponseBuilder, Body, DEFAULT_CHUNK_SIZE};
pub use self::responder::Responder;
pub use self::redirect::Redirect;
pub use self::flash::Flash;
pub use self::named_file::NamedFile;
pub use self::stream::Stream;
pub use self::failure::Failure;
#[doc(inline)] pub use self::content::Content;

/// Type alias for the `Result` of a `Responder::respond` call.
pub type Result<'r> = ::std::result::Result<self::Response<'r>, ::http::Status>;
