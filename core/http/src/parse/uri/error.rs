use std::{fmt, convert};
use std::borrow::Cow;

use pear::error::Expected;
use pear::input::ParseError;
use crate::parse::uri::RawInput;
use crate::ext::IntoOwned;

/// Error emitted on URI parse failure.
///
/// Internally, the type includes information about where the parse error
/// occurred (the error's context) and information about what went wrong.
/// Externally, this information can be retrieved (in textual form) through its
/// `Display` implementation. In other words, by printing a value of this type.
#[derive(Debug)]
pub struct Error<'a> {
    pub(crate) expected: Expected<u8, Cow<'a, [u8]>>,
    pub(crate) index: usize,
}

#[doc(hidden)]
impl<'a> From<ParseError<RawInput<'a>>> for Error<'a> {
    fn from(inner: ParseError<RawInput<'a>>) -> Self {
        let expected = inner.error.map(convert::identity, |v| v.values.into());
        Error { expected, index: inner.info.context.start }
    }
}

impl Error<'_> {
    /// Returns the byte index into the text where the error occurred if it is
    /// known.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::uri::Origin;
    ///
    /// let err = Origin::parse("/foo bar").unwrap_err();
    /// assert_eq!(err.index(), 4);
    /// ```
    pub fn index(&self) -> usize {
        self.index
    }
}

impl fmt::Display for Error<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at index {}", self.expected, self.index)
    }
}

impl IntoOwned for Error<'_> {
    type Owned = Error<'static>;

    fn into_owned(self) -> Error<'static> {
        Error {
            expected: self.expected.map(|t| t, |s| s.into_owned().into()),
            index: self.index
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::uri::origin_from_str;

    macro_rules! check_err {
        ($url:expr => $error:expr) => {{
            let e = origin_from_str($url).unwrap_err();
            assert_eq!(e.to_string(), $error.to_string())
        }}
    }

    #[test]
    fn check_display() {
        check_err!("a" => "expected token '/' but found 'a' at index 0");
        check_err!("?" => "expected token '/' but found '?' at index 0");
        check_err!("这" => "expected token '/' but found byte 232 at index 0");
    }
}
