//! Types for URIs and traits for rendering URI components.

#[macro_use]
mod uri;
mod origin;
mod reference;
mod authority;
mod absolute;
mod segments;
mod path_query;
mod asterisk;
mod host;

pub mod error;
pub mod fmt;

#[doc(inline)]
pub use self::error::Error;

pub use self::uri::*;
pub use self::authority::*;
pub use self::origin::*;
pub use self::absolute::*;
pub use self::segments::*;
pub use self::reference::*;
pub use self::path_query::*;
pub use self::asterisk::*;
pub use self::host::*;
