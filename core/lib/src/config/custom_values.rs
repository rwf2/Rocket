use std::fmt;

#[cfg(feature = "tls")] use crate::http::tls::{Certificate, PrivateKey};

use crate::http::private::Key;
use crate::config::{Result, Config, Value, ConfigError, LoggingLevel};
use crate::data::Limits;

#[derive(Clone)]
pub enum SecretKey {
    Generated(Key),
    Provided(Key)
}

impl SecretKey {
    #[inline]
    pub(crate) fn inner(&self) -> &Key {
        match *self {
            SecretKey::Generated(ref key) | SecretKey::Provided(ref key) => key
        }
    }

    #[inline]
    pub(crate) fn is_generated(&self) -> bool {
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
        limits = limits.limit(key.as_str(), val.into());
    }

    Ok(limits)
}
