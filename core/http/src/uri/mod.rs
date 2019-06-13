//! Types for URIs and traits for rendering URI components.

mod uri;
mod uri_display;
mod formatter;
mod from_uri_param;
mod origin;
mod authority;
mod absolute;
mod segments;

crate mod encoding;

pub use crate::parse::uri::Error;

pub use self::uri::*;
pub use self::authority::*;
pub use self::origin::*;
pub use self::absolute::*;
pub use self::uri_display::*;
pub use self::formatter::*;
pub use self::from_uri_param::*;
pub use self::segments::*;

mod private {
    pub trait Sealed {}
    impl Sealed for super::Path {}
    impl Sealed for super::Query {}
}

/// Marker trait for types that mark a part of a URI.
///
/// This trait exists solely to categorize types that mark a part of the URI,
/// currently [`Path`] and [`Query`]. Said another way, types that implement
/// this trait are marker types that represent a part of a URI at the
/// type-level.
///
/// This trait is _sealed_: it cannot be implemented outside of Rocket.
///
/// # Usage
///
/// You will find this trait in traits like [`UriDisplay`] or structs like
/// [`Formatter`] as the bound on a generic parameter: `P: UriPart`. Because the
/// trait is sealed, the generic type is guaranteed to be instantiated as one of
/// [`Query`] or [`Path`], effectively creating two instances of the generic
/// items: `UriDisplay<Query>` and `UriDisplay<Path>`, and `Formatter<Query>`
/// and `Formatter<Path>`. Unlike having two distinct, non-generic traits, this
/// approach enables succinct, type-checked generic implementations of these
/// items.
///
/// [`Query`]: crate::uri::Query
/// [`Path`]: crate::uri::Path
/// [`UriDisplay`]: crate::uri::UriDisplay
/// [`Formatter`]: crate::uri::Formatter
pub trait UriPart: private::Sealed {
    const DELIMITER: char;
}

/// Marker type indicating use of a type for the path [`UriPart`] of a URI.
///
/// In route URIs, this corresponds to all of the text before a `?`, if any, or
/// all of the text in the URI otherwise:
///
/// ```text
/// #[get("/home/<name>/<page>?<item>")]
///        ^------------------ Path
/// ```
///
/// [`UriPart`]: crate::uri::UriPart
#[derive(Debug, Clone, Copy)]
pub enum Path {  }

/// Marker type indicating use of a type for the query [`UriPart`] of a URI.
///
/// In route URIs, this corresponds to all of the text after a `?`, if any.
///
/// ```text
/// #[get("/home/<name>/<page>?<item>&<form..>")]
///                            ^-------------- Query
/// ```
///
/// [`UriPart`]: crate::uri::UriPart
#[derive(Debug, Clone, Copy)]
pub enum Query {  }

impl UriPart for Path {
    const DELIMITER: char = '/';
}

impl UriPart for Query {
    const DELIMITER: char = '&';
}
