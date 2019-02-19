//! Types and traits to build and send responses.
//!
//! The return type of a Rocket handler can be any type that implements the
//! [`Responder`](::response::Responder) trait, which means that the type knows
//! how to generate a [`Response`]. Among other things, this module contains
//! several such types.
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
//! The [`contrib` crate](rocket_contrib) contains several useful `Responder`s
//! including [`Template`](rocket_contrib::Template) and
//! [`Json`](rocket_contrib::Json).

mod named_file;
mod redirect;
mod responder;
mod response;
mod stream;

crate mod flash;

pub mod content;
pub mod status;

#[doc(inline)]
pub use self::content::Content;
pub use self::flash::Flash;
pub use self::named_file::NamedFile;
pub use self::redirect::Redirect;
pub use self::responder::Responder;
pub use self::response::{Body, Response, ResponseBuilder, DEFAULT_CHUNK_SIZE};
pub use self::stream::Stream;

/// Type alias for the `Result` of a `Responder::respond` call.
pub type Result<'r> = ::std::result::Result<self::Response<'r>, ::http::Status>;
