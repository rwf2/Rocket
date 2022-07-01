//! Errors arising from parsing invalid URIs.

use std::fmt;

pub use crate::parse::uri::Error;

/// The error type returned when a URI conversion fails.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TryFromUriError(pub(crate) ());

impl fmt::Display for TryFromUriError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "invalid conversion from general to specific URI variant".fmt(f)
    }
}

/// An error interpreting a segment as a [`PathBuf`] component in
/// [`Segments::to_path_buf()`].
///
/// [`PathBuf`]: std::path::PathBuf
/// [`Segments::to_path_buf()`]: crate::uri::Segments::to_path_buf()
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum PathError {
    /// The segment started with the wrapped invalid character.
    BadStart(char),
    /// The segment contained the wrapped invalid character.
    BadChar(char),
    /// The segment ended with the wrapped invalid character.
    BadEnd(char),
}
