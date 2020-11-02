use std::fmt;

use serde::{Serialize, Deserialize};
use crate::request::{Request, FromRequest, Outcome};

use crate::data::{ByteUnit, ToByteUnit};

/// Mapping from data types to read limits.
///
/// A `Limits` structure contains a mapping from a given data type ("forms",
/// "json", and so on) to the maximum size in bytes that should be accepted by a
/// Rocket application for that data type. For instance, if the limit for
/// "forms" is set to `256`, only 256 bytes from an incoming form request will
/// be read.
///
/// # Defaults
///
/// The default limits are:
///
///   * **forms**: 32KiB
///
/// # Usage
///
/// A `Limits` structure is created following the builder pattern:
///
/// ```rust
/// use rocket::data::{Limits, ToByteUnit};
///
/// // Set a limit of 64KiB for forms and 3MiB for JSON.
/// let limits = Limits::default()
///     .limit("forms", 64.kibibytes())
///     .limit("json", 3.mebibytes());
/// ```
///
/// The configured limits can be retrieved via the `&Limits` request guard:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// use std::io;
///
/// use rocket::data::{Data, Limits, ToByteUnit};
/// use rocket::response::Debug;
///
/// #[post("/echo", data = "<data>")]
/// async fn echo(data: Data, limits: &Limits) -> Result<String, Debug<io::Error>> {
///     let limit = limits.get("data").unwrap_or(1.mebibytes());
///     Ok(data.open(limit).stream_to_string().await?)
/// }
/// ```
///
/// ...or via the [`Request::limits()`] method:
///
/// ```
/// # #[macro_use] extern crate rocket;
/// use rocket::request::Request;
/// use rocket::data::{self, Data, FromData};
///
/// # struct MyType;
/// # type MyError = ();
/// #[rocket::async_trait]
/// impl FromData for MyType {
///     type Error = MyError;
///
///     async fn from_data(req: &Request<'_>, data: Data) -> data::Outcome<Self, MyError> {
///         let limit = req.limits().get("my-data-type");
///         /* .. */
///         # unimplemented!()
///     }
/// }
/// ```
#[serde(transparent)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Limits {
    // We cache this internally but don't share that fact in the API.
    #[serde(with = "figment::util::vec_tuple_map")]
    limits: Vec<(String, ByteUnit)>
}

/// The default limits are:
///
///   * **forms**: 32KiB
impl Default for Limits {
    fn default() -> Limits {
        // Default limit for forms is 32KiB.
        Limits { limits: vec![("forms".into(), 32.kibibytes())] }
    }
}

impl Limits {
    /// Construct a new `Limits` structure with no limits set.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::data::{Limits, ToByteUnit};
    ///
    /// let limits = Limits::default();
    /// assert_eq!(limits.get("forms"), Some(32.kibibytes()));
    ///
    /// let limits = Limits::new();
    /// assert_eq!(limits.get("forms"), None);
    /// ```
    #[inline]
    pub fn new() -> Self {
        Limits { limits: vec![] }
    }

    /// Adds or replaces a limit in `self`, consuming `self` and returning a new
    /// `Limits` structure with the added or replaced limit.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::data::{Limits, ToByteUnit};
    ///
    /// let limits = Limits::default().limit("json", 1.mebibytes());
    ///
    /// assert_eq!(limits.get("forms"), Some(32.kibibytes()));
    /// assert_eq!(limits.get("json"), Some(1.mebibytes()));
    ///
    /// let new_limits = limits.limit("json", 64.mebibytes());
    /// assert_eq!(new_limits.get("json"), Some(64.mebibytes()));
    /// ```
    pub fn limit<S: Into<String>>(mut self, name: S, limit: ByteUnit) -> Self {
        let name = name.into();
        match self.limits.iter_mut().find(|(k, _)| *k == name) {
            Some((_, v)) => *v = limit,
            None => self.limits.push((name, limit)),
        }

        self.limits.sort_by(|a, b| a.0.cmp(&b.0));
        self
    }

    /// Retrieve the set limit, if any, for the data type with name `name`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::data::{Limits, ToByteUnit};
    ///
    /// let limits = Limits::default().limit("json", 64.mebibytes());
    ///
    /// assert_eq!(limits.get("forms"), Some(32.kibibytes()));
    /// assert_eq!(limits.get("json"), Some(64.mebibytes()));
    /// assert!(limits.get("msgpack").is_none());
    /// ```
    pub fn get(&self, name: &str) -> Option<ByteUnit> {
        self.limits.iter()
            .find(|(k, _)| *k == name)
            .map(|(_, v)| *v)
    }
}

impl fmt::Display for Limits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, (k, v)) in self.limits.iter().enumerate() {
            if i != 0 { f.write_str(", ")? }
            write!(f, "{} = {}", k, v)?;
        }

        Ok(())
    }
}

#[crate::async_trait]
impl<'a, 'r> FromRequest<'a, 'r> for &'r Limits {
    type Error = std::convert::Infallible;

    async fn from_request(req: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        Outcome::Success(req.limits())
    }
}
