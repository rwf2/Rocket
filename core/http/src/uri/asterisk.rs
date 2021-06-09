use crate::ext::IntoOwned;
use crate::uri::Error;

/// The literal `*` URI.
///
/// # (De)serialization
///
/// `Asterisk` is both `Serialize` and `Deserialize`:
///
/// ```rust
/// # #[cfg(feature = "serde")] mod serde {
/// # use serde_ as serde;
/// use serde::{Serialize, Deserialize};
/// use rocket::http::uri::Asterisk;
///
/// #[derive(Deserialize, Serialize)]
/// # #[serde(crate = "serde_")]
/// struct UriOwned {
///     uri: Asterisk,
/// }
/// # }
/// ```
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub struct Asterisk;

impl Asterisk {
    /// Parses the string `string` into an `Asterisk`. Parsing will never
    /// allocate. Returns an `Error` if `string` is not a valid asterisk URI.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::uri::Asterisk;
    ///
    /// assert!(Asterisk::parse("*").is_ok());
    /// assert!(Asterisk::parse("/foo/bar").is_err());
    ///
    /// // Prefer to use `uri!()` when the input is statically known:
    /// let uri = uri!("*");
    /// assert_eq!(uri, Asterisk);
    /// ```
    pub fn parse(string: &str) -> Result<Asterisk, Error<'_>> {
        crate::parse::uri::asterisk_from_str(string)
    }

    /// Parses the string `string` into an `Asterisk`. This is equivalent to
    /// [`Asterisk::parse()`].
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::uri::Asterisk;
    ///
    /// assert!(Asterisk::parse_owned("*".to_string()).is_ok());
    /// assert!(Asterisk::parse_owned("/foo/bar".to_string()).is_err());
    /// ```
    pub fn parse_owned(string: String) -> Result<Asterisk, Error<'static>> {
        Asterisk::parse(&string).map_err(|e| e.into_owned())
    }
}

impl std::fmt::Display for Asterisk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        "*".fmt(f)
    }
}

impl_serde!(Asterisk, "an asterisk-form URI, '*'");
