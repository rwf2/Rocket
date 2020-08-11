use std::fmt;

use crate::data::{ByteUnit, ToByteUnit};

/// Mapping from data type to size limits.
///
/// A `Limits` structure contains a mapping from a given data type ("forms",
/// "json", and so on) to the maximum size in bytes that should be accepted by a
/// Rocket application for that data type. For instance, if the limit for
/// "forms" is set to `256`, only 256 bytes from an incoming form request will
/// be read.
///
/// # Defaults
///
/// As documented in [`config`](crate::config), the default limits are as follows:
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
/// let limits = Limits::new()
///     .limit("forms", 64.kibibytes())
///     .limit("json", 3.mebibytes());
/// ```
#[derive(Debug, Clone)]
pub struct Limits {
    // We cache this internally but don't share that fact in the API.
    pub(crate) forms: ByteUnit,
    extra: Vec<(String, ByteUnit)>
}

impl Default for Limits {
    fn default() -> Limits {
        // Default limit for forms is 32KiB.
        Limits { forms: 32.kibibytes(), extra: Vec::new() }
    }
}

impl Limits {
    /// Construct a new `Limits` structure with the default limits set.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::data::{Limits, ToByteUnit};
    ///
    /// let limits = Limits::new();
    /// assert_eq!(limits.get("forms"), Some(32.kibibytes()));
    /// ```
    #[inline]
    pub fn new() -> Self {
        Limits::default()
    }

    /// Adds or replaces a limit in `self`, consuming `self` and returning a new
    /// `Limits` structure with the added or replaced limit.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::data::{Limits, ToByteUnit};
    ///
    /// let limits = Limits::new().limit("json", 1.mebibytes());
    ///
    /// assert_eq!(limits.get("forms"), Some(32.kibibytes()));
    /// assert_eq!(limits.get("json"), Some(1.mebibytes()));
    ///
    /// let new_limits = limits.limit("json", 64.mebibytes());
    /// assert_eq!(new_limits.get("json"), Some(64.mebibytes()));
    /// ```
    pub fn limit<S: Into<String>>(mut self, name: S, limit: ByteUnit) -> Self {
        let name = name.into();
        match name.as_str() {
            "forms" => self.forms = limit,
            _ => {
                let mut found = false;
                for tuple in &mut self.extra {
                    if tuple.0 == name {
                        tuple.1 = limit;
                        found = true;
                        break;
                    }
                }

                if !found {
                    self.extra.push((name, limit))
                }
            }
        }

        self
    }

    /// Retrieve the set limit, if any, for the data type with name `name`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::data::{Limits, ToByteUnit};
    ///
    /// let limits = Limits::new().limit("json", 64.mebibytes());
    ///
    /// assert_eq!(limits.get("forms"), Some(32.kibibytes()));
    /// assert_eq!(limits.get("json"), Some(64.mebibytes()));
    /// assert!(limits.get("msgpack").is_none());
    /// ```
    pub fn get(&self, name: &str) -> Option<ByteUnit> {
        if name == "forms" {
            return Some(self.forms);
        }

        for &(ref key, val) in &self.extra {
            if key == name {
                return Some(val);
            }
        }

        None
    }
}

impl fmt::Display for Limits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "forms = {}", self.forms)?;
        for (key, val) in &self.extra {
            write!(f, ", {}* = {}", key, val)?;
        }

        Ok(())
    }
}

