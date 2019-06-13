use std::fmt;

#[cfg(feature = "tls")] use crate::http::tls::{Certificate, PrivateKey};

use crate::http::private::Key;
use crate::config::{Result, Config, Value, ConfigError, LoggingLevel};

#[derive(Clone)]
pub enum SecretKey {
    Generated(Key),
    Provided(Key)
}

impl SecretKey {
    #[inline]
    crate fn inner(&self) -> &Key {
        match *self {
            SecretKey::Generated(ref key) | SecretKey::Provided(ref key) => key
        }
    }

    #[inline]
    crate fn is_generated(&self) -> bool {
        match *self {
            #[cfg(feature = "private-cookies")]
            SecretKey::Generated(_) => true,
            _ => false
        }
    }
}

impl fmt::Display for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[cfg(feature = "private-cookies")]
        match *self {
            SecretKey::Generated(_) => write!(f, "generated"),
            SecretKey::Provided(_) => write!(f, "provided"),
        }

        #[cfg(not(feature = "private-cookies"))]
        write!(f, "private-cookies disabled")
    }
}

#[cfg(feature = "tls")]
#[derive(Clone)]
pub struct TlsConfig {
    pub certs: Vec<Certificate>,
    pub key: PrivateKey
}

#[cfg(not(feature = "tls"))]
#[derive(Clone)]
pub struct TlsConfig;

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
/// use rocket::config::Limits;
///
/// // Set a limit of 64KiB for forms and 3MiB for JSON.
/// let limits = Limits::new()
///     .limit("forms", 64 * 1024)
///     .limit("json", 3 * 1024 * 1024);
/// ```
#[derive(Debug, Clone)]
pub struct Limits {
    // We cache this internally but don't share that fact in the API.
    crate forms: u64,
    extra: Vec<(String, u64)>
}

impl Default for Limits {
    fn default() -> Limits {
        // Default limit for forms is 32KiB.
        Limits { forms: 32 * 1024, extra: Vec::new() }
    }
}

impl Limits {
    /// Construct a new `Limits` structure with the default limits set.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::Limits;
    ///
    /// let limits = Limits::new();
    /// assert_eq!(limits.get("forms"), Some(32 * 1024));
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
    /// use rocket::config::Limits;
    ///
    /// let limits = Limits::new()
    ///     .limit("json", 1 * 1024 * 1024);
    ///
    /// assert_eq!(limits.get("forms"), Some(32 * 1024));
    /// assert_eq!(limits.get("json"), Some(1 * 1024 * 1024));
    ///
    /// let new_limits = limits.limit("json", 64 * 1024 * 1024);
    /// assert_eq!(new_limits.get("json"), Some(64 * 1024 * 1024));
    /// ```
    pub fn limit<S: Into<String>>(mut self, name: S, limit: u64) -> Self {
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
    /// use rocket::config::Limits;
    ///
    /// let limits = Limits::new()
    ///     .limit("json", 64 * 1024 * 1024);
    ///
    /// assert_eq!(limits.get("forms"), Some(32 * 1024));
    /// assert_eq!(limits.get("json"), Some(64 * 1024 * 1024));
    /// assert!(limits.get("msgpack").is_none());
    /// ```
    pub fn get(&self, name: &str) -> Option<u64> {
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
        fn fmt_size(n: u64, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            if (n & ((1 << 20) - 1)) == 0 {
                write!(f, "{}MiB", n >> 20)
            } else if (n & ((1 << 10) - 1)) == 0 {
                write!(f, "{}KiB", n >> 10)
            } else {
                write!(f, "{}B", n)
            }
        }

        write!(f, "forms = ")?;
        fmt_size(self.forms, f)?;
        for &(ref key, val) in &self.extra {
            write!(f, ", {}* = ", key)?;
            fmt_size(val, f)?;
        }

        Ok(())
    }
}

pub fn str<'a>(conf: &Config, name: &str, v: &'a Value) -> Result<&'a str> {
    v.as_str().ok_or_else(|| conf.bad_type(name, v.type_str(), "a string"))
}

pub fn u64(conf: &Config, name: &str, value: &Value) -> Result<u64> {
    match value.as_integer() {
        Some(x) if x >= 0 => Ok(x as u64),
        _ => Err(conf.bad_type(name, value.type_str(), "an unsigned integer"))
    }
}

pub fn u16(conf: &Config, name: &str, value: &Value) -> Result<u16> {
    match value.as_integer() {
        Some(x) if x >= 0 && x <= (u16::max_value() as i64) => Ok(x as u16),
        _ => Err(conf.bad_type(name, value.type_str(), "a 16-bit unsigned integer"))
    }
}

pub fn u32(conf: &Config, name: &str, value: &Value) -> Result<u32> {
    match value.as_integer() {
        Some(x) if x >= 0 && x <= (u32::max_value() as i64) => Ok(x as u32),
        _ => Err(conf.bad_type(name, value.type_str(), "a 32-bit unsigned integer"))
    }
}

pub fn log_level(conf: &Config,
                          name: &str,
                          value: &Value
                         ) -> Result<LoggingLevel> {
    str(conf, name, value)
        .and_then(|s| s.parse().map_err(|e| conf.bad_type(name, value.type_str(), e)))
}

pub fn tls_config<'v>(conf: &Config,
                               name: &str,
                               value: &'v Value,
                               ) -> Result<(&'v str, &'v str)> {
    let (mut certs_path, mut key_path) = (None, None);
    let table = value.as_table()
        .ok_or_else(|| conf.bad_type(name, value.type_str(), "a table"))?;

    let env = conf.environment;
    for (key, value) in table {
        match key.as_str() {
            "certs" => certs_path = Some(str(conf, "tls.certs", value)?),
            "key" => key_path = Some(str(conf, "tls.key", value)?),
            _ => return Err(ConfigError::UnknownKey(format!("{}.tls.{}", env, key)))
        }
    }

    if let (Some(certs), Some(key)) = (certs_path, key_path) {
        Ok((certs, key))
    } else {
        Err(conf.bad_type(name, "a table with missing entries",
                            "a table with `certs` and `key` entries"))
    }
}

pub fn limits(conf: &Config, name: &str, value: &Value) -> Result<Limits> {
    let table = value.as_table()
        .ok_or_else(|| conf.bad_type(name, value.type_str(), "a table"))?;

    let mut limits = Limits::default();
    for (key, val) in table {
        let val = u64(conf, &format!("limits.{}", key), val)?;
        limits = limits.limit(key.as_str(), val);
    }

    Ok(limits)
}
