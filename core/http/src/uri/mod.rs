//! Types for URIs and traits for rendering URI components.

#[macro_use]
mod uri;
mod absolute;
mod asterisk;
mod authority;
mod host;
mod origin;
mod path_query;
mod reference;
mod segments;

pub mod error;
pub mod fmt;

#[doc(inline)]
pub use self::error::Error;

pub use self::absolute::*;
pub use self::asterisk::*;
pub use self::authority::*;
pub use self::host::*;
pub use self::origin::*;
pub use self::path_query::*;
pub use self::reference::*;
pub use self::segments::*;
pub use self::uri::*;
