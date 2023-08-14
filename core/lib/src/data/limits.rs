use std::fmt;

use serde::{Serialize, Deserialize};
use crate::request::{Request, FromRequest, Outcome};

use crate::data::ByteUnit;
use crate::http::uncased::Uncased;

/// Mapping from (hierarchical) data types to size limits.
///
/// A `Limits` structure contains a mapping from a given hierarchical data type
/// ("form", "data-form", "ext/pdf", and so on) to the maximum size in bytes
/// that should be accepted by Rocket for said data type. For instance, if the
/// limit for "form" is set to `256`, only 256 bytes from an incoming non-data
/// form (that is, url-encoded) will be accepted.
///
/// To help in preventing DoS attacks, all incoming data reads must capped by a
/// limit. As such, all data guards impose a limit. The _name_ of the limit is
/// dictated by the data guard or type itself. For instance, [`Form`] imposes
/// the `form` limit for value-based forms and `data-form` limit for data-based
/// forms.
///
/// If a limit is exceeded, a guard will typically fail. The [`Capped`] type
/// allows retrieving some data types even when the limit is exceeded.
///
/// [`Capped`]: crate::data::Capped
/// [`Form`]: crate::form::Form
///
/// # Hierarchy
///
/// Data limits are hierarchical. The `/` (forward slash) character delimits the
/// levels, or layers, of a given limit. To obtain a limit value for a given
/// name, layers are peeled from right to left until a match is found, if any.
/// For example, fetching the limit named `pet/dog/bingo` will return the first
/// of `pet/dog/bingo`, `pet/dog` or `pet`:
///
/// ```rust
/// use rocket::data::{Limits, ToByteUnit};
///
/// let limits = Limits::default()
///     .limit("pet", 64.kibibytes())
///     .limit("pet/dog", 128.kibibytes())
///     .limit("pet/dog/bingo", 96.kibibytes());
///
/// assert_eq!(limits.get("pet/dog/bingo"), Some(96.kibibytes()));
/// assert_eq!(limits.get("pet/dog/ralph"), Some(128.kibibytes()));
/// assert_eq!(limits.get("pet/cat/bingo"), Some(64.kibibytes()));
///
/// assert_eq!(limits.get("pet/dog/bingo/hat"), Some(96.kibibytes()));
/// ```
///
/// # Built-in Limits
///
/// The following table details recognized built-in limits used by Rocket.
///
/// | Limit Name        | Default | Type         | Description                           |
/// |-------------------|---------|--------------|---------------------------------------|
/// | `form`            | 32KiB   | [`Form`]     | entire non-data-based form            |
/// | `data-form`       | 2MiB    | [`Form`]     | entire data-based form                |
/// | `file`            | 1MiB    | [`TempFile`] | [`TempFile`] data guard or form field |
/// | `file/$ext`       | _N/A_   | [`TempFile`] | file form field with extension `$ext` |
/// | `string`          | 8KiB    | [`String`]   | data guard or data form field         |
/// | `bytes`           | 8KiB    | [`Vec<u8>`]  | data guard                            |
/// | `json`            | 1MiB    | [`Json`]     | JSON data and form payloads           |
/// | `msgpack`         | 1MiB    | [`MsgPack`]  | MessagePack data and form payloads    |
///
/// [`TempFile`]: crate::fs::TempFile
/// [`Json`]: crate::serde::json::Json
/// [`MsgPack`]: crate::serde::msgpack::MsgPack
///
/// # Usage
///
/// A `Limits` structure is created following the builder pattern:
///
/// ```rust
/// use rocket::data::{Limits, ToByteUnit};
///
/// // Set a limit of 64KiB for forms, 3MiB for PDFs, and 1MiB for JSON.
/// let limits = Limits::default()
///     .limit("form", 64.kibibytes())
///     .limit("file/pdf", 3.mebibytes())
///     .limit("json", 2.mebibytes());
/// ```
///
/// The [`Limits::default()`](#impl-Default) method populates the `Limits`
/// structure with default limits in the [table above](#built-in-limits). A
/// configured limit can be retrieved via the `&Limits` request guard:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// use std::io;
///
/// use rocket::data::{Data, Limits, ToByteUnit};
///
/// #[post("/echo", data = "<data>")]
/// async fn echo(data: Data<'_>, limits: &Limits) -> io::Result<String> {
///     let limit = limits.get("data").unwrap_or(1.mebibytes());
///     Ok(data.open(limit).into_string().await?.value)
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
/// impl<'r> FromData<'r> for MyType {
///     type Error = MyError;
///
///     async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> data::Outcome<'r, Self> {
///         let limit = req.limits().get("my-data-type");
///         /* .. */
///         # unimplemented!()
///     }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Limits {
    #[serde(deserialize_with = "Limits::deserialize")]
    #[serde(serialize_with = "figment::util::vec_tuple_map::serialize")]
    limits: Vec<(Uncased<'static>, ByteUnit)>,
}

impl Default for Limits {
    fn default() -> Limits {
        Limits::new()
            .limit("form", Limits::FORM)
            .limit("data-form", Limits::DATA_FORM)
            .limit("file", Limits::FILE)
            .limit("string", Limits::STRING)
            .limit("bytes", Limits::BYTES)
            .limit("json", Limits::JSON)
            .limit("msgpack", Limits::MESSAGE_PACK)
    }
}

impl Limits {
    /// Default limit for value-based forms.
    pub const FORM: ByteUnit = ByteUnit::Kibibyte(32);

    /// Default limit for data-based forms.
    pub const DATA_FORM: ByteUnit = ByteUnit::Mebibyte(2);

    /// Default limit for temporary files.
    pub const FILE: ByteUnit = ByteUnit::Mebibyte(1);

    /// Default limit for strings.
    pub const STRING: ByteUnit = ByteUnit::Kibibyte(8);

    /// Default limit for bytes.
    pub const BYTES: ByteUnit = ByteUnit::Kibibyte(8);

    /// Default limit for JSON payloads.
    pub const JSON: ByteUnit = ByteUnit::Mebibyte(1);

    /// Default limit for MessagePack payloads.
    pub const MESSAGE_PACK: ByteUnit = ByteUnit::Mebibyte(1);

    /// Construct a new `Limits` structure with no limits set.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::data::{Limits, ToByteUnit};
    ///
    /// let limits = Limits::default();
    /// assert_eq!(limits.get("form"), Some(32.kibibytes()));
    ///
    /// let limits = Limits::new();
    /// assert_eq!(limits.get("form"), None);
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
    /// let limits = Limits::default();
    /// assert_eq!(limits.get("form"), Some(32.kibibytes()));
    /// assert_eq!(limits.get("json"), Some(1.mebibytes()));
    /// assert_eq!(limits.get("cat"), None);
    ///
    /// let limits = limits.limit("cat", 1.mebibytes());
    /// assert_eq!(limits.get("form"), Some(32.kibibytes()));
    /// assert_eq!(limits.get("cat"), Some(1.mebibytes()));
    ///
    /// let limits = limits.limit("json", 64.mebibytes());
    /// assert_eq!(limits.get("json"), Some(64.mebibytes()));
    /// ```
    pub fn limit<S: Into<Uncased<'static>>>(mut self, name: S, limit: ByteUnit) -> Self {
        let name = name.into();
        match self.limits.binary_search_by(|(k, _)| k.cmp(&name)) {
            Ok(i) => self.limits[i].1 = limit,
            Err(i) => self.limits.insert(i, (name, limit))
        }

        self
    }

    /// Returns the limit named `name`, proceeding hierarchically from right
    /// to left until one is found, or returning `None` if none is found.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::data::{Limits, ToByteUnit};
    ///
    /// let limits = Limits::default()
    ///     .limit("json", 2.mebibytes())
    ///     .limit("file/jpeg", 4.mebibytes())
    ///     .limit("file/jpeg/special", 8.mebibytes());
    ///
    /// assert_eq!(limits.get("form"), Some(32.kibibytes()));
    /// assert_eq!(limits.get("json"), Some(2.mebibytes()));
    /// assert_eq!(limits.get("data-form"), Some(Limits::DATA_FORM));
    ///
    /// assert_eq!(limits.get("file"), Some(1.mebibytes()));
    /// assert_eq!(limits.get("file/png"), Some(1.mebibytes()));
    /// assert_eq!(limits.get("file/jpeg"), Some(4.mebibytes()));
    /// assert_eq!(limits.get("file/jpeg/inner"), Some(4.mebibytes()));
    /// assert_eq!(limits.get("file/jpeg/special"), Some(8.mebibytes()));
    ///
    /// assert!(limits.get("cats").is_none());
    /// ```
    pub fn get<S: AsRef<str>>(&self, name: S) -> Option<ByteUnit> {
        let mut name = name.as_ref();
        let mut indices = name.rmatch_indices('/');
        loop {
            let exact_limit = self.limits
                .binary_search_by(|(k, _)| k.as_uncased_str().cmp(name.into()))
                .map(|i| self.limits[i].1);

            if let Ok(exact) = exact_limit {
                return Some(exact);
            }

            let (i, _) = indices.next()?;
            name = &name[..i];
        }
    }

    /// Returns the limit for the name created by joining the strings in
    /// `layers` with `/` as a separator, then proceeding like
    /// [`Limits::get()`], hierarchically from right to left until one is found,
    /// or returning `None` if none is found.
    ///
    /// This methods exists to allow finding hierarchical limits without
    /// constructing a string to call `get()` with but otherwise returns the
    /// same results.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::data::{Limits, ToByteUnit};
    ///
    /// let limits = Limits::default()
    ///     .limit("json", 2.mebibytes())
    ///     .limit("file/jpeg", 4.mebibytes())
    ///     .limit("file/jpeg/special", 8.mebibytes());
    ///
    /// assert_eq!(limits.find(["json"]), Some(2.mebibytes()));
    /// assert_eq!(limits.find(["json", "person"]), Some(2.mebibytes()));
    ///
    /// assert_eq!(limits.find(["file"]), Some(1.mebibytes()));
    /// assert_eq!(limits.find(["file", "png"]), Some(1.mebibytes()));
    /// assert_eq!(limits.find(["file", "jpeg"]), Some(4.mebibytes()));
    /// assert_eq!(limits.find(["file", "jpeg", "inner"]), Some(4.mebibytes()));
    /// assert_eq!(limits.find(["file", "jpeg", "special"]), Some(8.mebibytes()));
    ///
    /// # let s: &[&str] = &[]; assert_eq!(limits.find(s), None);
    /// ```
    pub fn find<S: AsRef<str>, L: AsRef<[S]>>(&self, layers: L) -> Option<ByteUnit> {
        let layers = layers.as_ref();
        for j in (1..=layers.len()).rev() {
            let layers = &layers[..j];
            let opt = self.limits
                .binary_search_by(|(k, _)| {
                    let k_layers = k.as_str().split('/');
                    k_layers.cmp(layers.iter().map(|s| s.as_ref()))
                })
                .map(|i| self.limits[i].1);

            if let Ok(byte_unit) = opt {
                return Some(byte_unit);
            }
        }

        None
    }

    /// Deserialize a `Limits` vector from a map. Ensures that the resulting
    /// vector is properly sorted for futures lookups via binary search.
    fn deserialize<'de, D>(de: D) -> Result<Vec<(Uncased<'static>, ByteUnit)>, D::Error>
        where D: serde::Deserializer<'de>
    {
        let mut limits = figment::util::vec_tuple_map::deserialize(de)?;
        limits.sort();
        Ok(limits)
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
impl<'r> FromRequest<'r> for &'r Limits {
    type Error = std::convert::Infallible;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        Outcome::Success(req.limits())
    }
}
