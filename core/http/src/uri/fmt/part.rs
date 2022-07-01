use crate::parse::IndexedStr;

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
/// [`Formatter`] as the bound on a generic parameter: `P: Part`. Because the
/// trait is sealed, the generic type is guaranteed to be instantiated as one of
/// [`Query`] or [`Path`], effectively creating two instances of the generic
/// items: `UriDisplay<Query>` and `UriDisplay<Path>`, and `Formatter<Query>`
/// and `Formatter<Path>`. Unlike having two distinct, non-generic traits, this
/// approach enables succinct, type-checked generic implementations of these
/// items.
///
/// [`UriDisplay`]: crate::uri::fmt::UriDisplay
/// [`Formatter`]: crate::uri::fmt::Formatter
pub trait Part: private::Sealed {
    /// The dynamic version of `Self`.
    #[doc(hidden)]
    const KIND: Kind;

    /// The delimiter used to separate components of this URI part.
    /// Specifically, `/` for `Path` and `&` for `Query`.
    #[doc(hidden)]
    const DELIMITER: char;

    /// The raw form of a segment in this part.
    #[doc(hidden)]
    type Raw: Send + Sync + 'static;
}

mod private {
    pub trait Sealed {}
    impl Sealed for super::Path {}
    impl Sealed for super::Query {}
}

/// Dynamic version of the `Path` and `Query` parts.
#[doc(hidden)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Kind { Path, Query }

/// Marker type indicating use of a type for the path [`Part`] of a URI.
///
/// In route URIs, this corresponds to all of the text before a `?`, if any, or
/// all of the text in the URI otherwise:
///
/// ```text
/// #[get("/home/<name>/<page>?<item>")]
///        ^------------------ Path
/// ```
#[derive(Debug, Clone, Copy)]
pub enum Path {  }

/// Marker type indicating use of a type for the query [`Part`] of a URI.
///
/// In route URIs, this corresponds to all of the text after a `?`, if any.
///
/// ```text
/// #[get("/home/<name>/<page>?<item>&<form..>")]
///                            ^-------------- Query
/// ```
#[derive(Debug, Clone, Copy)]
pub enum Query {  }

impl Part for Path {
    const KIND: Kind = Kind::Path;
    const DELIMITER: char = '/';
    type Raw = IndexedStr<'static>;
}

impl Part for Query {
    const KIND: Kind = Kind::Query;
    const DELIMITER: char = '&';
    type Raw = (IndexedStr<'static>, IndexedStr<'static>);
}
