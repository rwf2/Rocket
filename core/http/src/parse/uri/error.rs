use std::fmt;
use std::borrow::Cow;

use pear::{ParseErr, Expected};
use crate::parse::indexed::Context;
use crate::parse::uri::RawInput;
use crate::ext::IntoOwned;

/// Error emitted on URI parse failure.
///
/// Internally, the type includes information about where the parse error
/// occured (the error's context) and information about what went wrong.
/// Externally, this information can be retrieved (in textual form) through its
/// `Display` implementation. In other words, by printing a value of this type.
#[derive(Debug)]
pub struct Error<'a> {
    expected: Expected<Or<char, u8>, Cow<'a, str>, String>,
    context: Option<Context>
}

#[derive(Debug)]
enum Or<L, R> {
    A(L),
    B(R)
}

impl<'a> Error<'a> {
    crate fn from(src: &'a str, pear_error: ParseErr<RawInput<'a>>) -> Error<'a> {
        let new_expected = pear_error.expected.map(|token| {
            if token.is_ascii() && !token.is_ascii_control() {
                Or::A(token as char)
            } else {
                Or::B(token)
            }
        }, String::from_utf8_lossy, |indexed| {
            let src = Some(src.as_bytes());
            String::from_utf8_lossy(indexed.from_source(src)).to_string()
        });

        Error { expected: new_expected, context: pear_error.context }
    }

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
    /// assert_eq!(err.index(), Some(4));
    /// ```
    pub fn index(&self) -> Option<usize> {
        self.context.as_ref().map(|c| c.offset)
    }
}

impl fmt::Display for Or<char, u8> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Or::A(left) => write!(f, "'{}'", left),
            Or::B(right) => write!(f, "non-ASCII byte {}", right),
        }
    }
}

impl fmt::Display for Error<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // This relies on specialization of the `Display` impl for `Expected`.
        write!(f, "{}", self.expected)?;

        if let Some(ref context) = self.context {
            write!(f, " at index {}", context.offset)?;
        }

        Ok(())
    }
}

impl IntoOwned for Error<'_> {
    type Owned = Error<'static>;

    fn into_owned(self) -> Self::Owned {
        let expected = self.expected.map(|i| i, IntoOwned::into_owned, |i| i);
        Error { expected, context: self.context }
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
        check_err!("这" => "expected token '/' but found non-ASCII byte 232 at index 0");
    }
}
