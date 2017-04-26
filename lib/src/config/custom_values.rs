use std::fmt;

#[cfg(feature = "tls")] use rustls::{Certificate, PrivateKey};

use logger::LoggingLevel;
use config::{Result, Config, Value, ConfigError};
use http::Key;

#[derive(Clone)]
pub enum SessionKey {
    Generated(Key),
    Provided(Key)
}

impl SessionKey {
    #[inline(always)]
    pub fn kind(&self) -> &'static str {
        match *self {
            SessionKey::Generated(_) => "generated",
            SessionKey::Provided(_) => "provided",
        }
    }

    #[inline(always)]
    pub(crate) fn inner(&self) -> &Key {
        match *self {
            SessionKey::Generated(ref key) | SessionKey::Provided(ref key) => key
        }
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

// Size limit configuration. We cache those used by Rocket internally but don't
// share that fact in the API.
#[derive(Debug, Clone)]
pub struct Limits {
    pub(crate) forms: u64,
    extra: Vec<(String, u64)>
}

impl Default for Limits {
    fn default() -> Limits {
        Limits { forms: 1024 * 32, extra: Vec::new() }
    }
}

impl Limits {
    pub fn add<S: Into<String>>(mut self, name: S, limit: u64) -> Self {
        let name = name.into();
        match name.as_str() {
            "forms" => self.forms = limit,
            _ => self.extra.push((name, limit))
        }

        self
    }

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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fn fmt_size(n: u64, f: &mut fmt::Formatter) -> fmt::Result {
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

pub fn value_as_str<'a>(conf: &Config, name: &str, v: &'a Value) -> Result<&'a str> {
    v.as_str().ok_or(conf.bad_type(name, v.type_str(), "a string"))
}

pub fn value_as_u64(conf: &Config, name: &str, value: &Value) -> Result<u64> {
    match value.as_integer() {
        Some(x) if x >= 0 => Ok(x as u64),
        _ => Err(conf.bad_type(name, value.type_str(), "an unsigned integer"))
    }
}

pub fn value_as_u16(conf: &Config, name: &str, value: &Value) -> Result<u16> {
    match value.as_integer() {
        Some(x) if x >= 0 && x <= (u16::max_value() as i64) => Ok(x as u16),
        _ => Err(conf.bad_type(name, value.type_str(), "a 16-bit unsigned integer"))
    }
}

pub fn value_as_log_level(conf: &Config,
                          name: &str,
                          value: &Value
                         ) -> Result<LoggingLevel> {
    value_as_str(conf, name, value)
        .and_then(|s| s.parse().map_err(|e| conf.bad_type(name, value.type_str(), e)))
}

pub fn value_as_tls_config<'v>(conf: &Config,
                               name: &str,
                               value: &'v Value,
                               ) -> Result<(&'v str, &'v str)> {
    let (mut certs_path, mut key_path) = (None, None);
    let table = value.as_table()
        .ok_or_else(|| conf.bad_type(name, value.type_str(), "a table"))?;

    let env = conf.environment;
    for (key, value) in table {
        match key.as_str() {
            "certs" => certs_path = Some(value_as_str(conf, "tls.certs", value)?),
            "key" => key_path = Some(value_as_str(conf, "tls.key", value)?),
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

pub fn value_as_limits(conf: &Config, name: &str, value: &Value) -> Result<Limits> {
    let table = value.as_table()
        .ok_or_else(|| conf.bad_type(name, value.type_str(), "a table"))?;

    let mut limits = Limits::default();
    for (key, val) in table {
        let val = value_as_u64(conf, &format!("limits.{}", key), val)?;
        limits = limits.add(key.as_str(), val);
    }

    Ok(limits)
}
