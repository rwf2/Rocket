use std::collections::HashMap;
use std::net::ToSocketAddrs;
use std::path::Path;
use std::sync::RwLock;
use std::fmt;

use config::Environment::*;
use config::{self, Environment, ConfigError};

use logger::LoggingLevel;
use toml::Value;

/// The core configuration structure.
pub struct Config {
    /// The address to serve on.
    pub address: String,
    /// The port to serve on.
    pub port: u16,
    /// How much information to log.
    pub log_level: LoggingLevel,
    /// The environment that this configuration corresponds to.
    pub env: Environment,
    session_key: RwLock<Option<String>>,
    extras: HashMap<String, Value>,
    filepath: String,
}

macro_rules! parse {
    ($conf:expr, $name:expr, $val:expr, $method:ident, $expect: expr) => (
        $val.$method().ok_or_else(|| {
            $conf.bad_type($name, $val, $expect)
        })
    );
}

impl Config {
    /// Returns the default configuration for the environment `env` given that
    /// the configuration was stored at `filepath`. If `filepath` is not an
    /// absolute path, an `Err` of `ConfigError::BadFilePath` is returned.
    pub fn default_for(env: Environment, filepath: &str) -> config::Result<Config> {
        let file_path = Path::new(filepath);
        if file_path.parent().is_none() {
            return Err(ConfigError::BadFilePath(filepath.to_string(),
                "Configuration files must be rooted in a directory."));
        }

        Ok(match env {
            Development => {
                Config {
                    address: "localhost".to_string(),
                    port: 8000,
                    log_level: LoggingLevel::Normal,
                    session_key: RwLock::new(None),
                    extras: HashMap::new(),
                    env: env,
                    filepath: filepath.to_string(),
                }
            }
            Staging => {
                Config {
                    address: "0.0.0.0".to_string(),
                    port: 80,
                    log_level: LoggingLevel::Normal,
                    session_key: RwLock::new(None),
                    extras: HashMap::new(),
                    env: env,
                    filepath: filepath.to_string(),
                }
            }
            Production => {
                Config {
                    address: "0.0.0.0".to_string(),
                    port: 80,
                    log_level: LoggingLevel::Critical,
                    session_key: RwLock::new(None),
                    extras: HashMap::new(),
                    env: env,
                    filepath: filepath.to_string(),
                }
            }
        })
    }

    /// Constructs a `BadType` error given the entry `name`, the invalid `val`
    /// at that entry, and the `expect`ed type name.
    #[inline(always)]
    fn bad_type(&self, name: &str, val: &Value, expect: &'static str) -> ConfigError {
        let id = format!("{}.{}", self.env, name);
        ConfigError::BadType(id, expect, val.type_str(), self.filepath.clone())
    }

    /// Sets the configuration `val` for the `name` entry. If the `name` is one
    /// of "address", "port", "session_key", or "log" (the "default" values),
    /// the appropriate value in the `self` Config structure is set. Otherwise,
    /// the value is stored as an `extra`.
    ///
    /// For each of the default values, the following `Value` variant is
    /// expected. If a different variant is supplied, a `BadType` `Err` is
    /// returned:
    ///
    ///   * **address**: String
    ///   * **port**: Integer (16-bit unsigned)
    ///   * **session_key**: String (192-bit base64)
    ///   * **log**: String
    ///
    pub fn set(&mut self, name: &str, val: &Value) -> config::Result<()> {
        if name == "address" {
            let address_str = parse!(self, name, val, as_str, "a string")?;
            if address_str.contains(':') {
                return Err(self.bad_type(name, val, "an IP address with no port"));
            } else if format!("{}:{}", address_str, 80).to_socket_addrs().is_err() {
                return Err(self.bad_type(name, val, "a valid IP address"));
            }

            self.address = address_str.to_string();
        } else if name == "port" {
            let port = parse!(self, name, val, as_integer, "an integer")?;
            if port < 0 {
                return Err(self.bad_type(name, val, "an unsigned integer"));
            }

            if port > (u16::max_value() as i64) {
                return Err(self.bad_type(name, val, "a 16-bit unsigned integer"))
            }

            self.port = port as u16;
        } else if name == "session_key" {
            let key = parse!(self, name, val, as_str, "a string")?;
            if key.len() != 32 {
                return Err(self.bad_type(name, val, "a 192-bit base64 string"));
            }

            self.session_key = RwLock::new(Some(key.to_string()));
        } else if name == "log" {
            let level_str = parse!(self, name, val, as_str, "a string")?;
            self.log_level = match level_str.parse() {
                Ok(level) => level,
                Err(_) => return Err(self.bad_type(name, val,
                                "log level ('normal', 'critical', 'debug')"))
            };
        } else {
            self.extras.insert(name.into(), val.clone());
        }

        Ok(())
    }

    /// Moves the session key string out of the `self` Config, if there is one.
    /// Because the value is moved out, subsequent calls will result in a return
    /// value of `None`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::{Config, Environment, Value};
    ///
    /// // Create a new config with a session key.
    /// let key = "adL5fFIPmZBrlyHk2YT4NLV3YCk2gFXz".to_string();
    /// let config = Config::default_for(Environment::Staging, "/custom").unwrap()
    ///     .session_key(key.clone());
    ///
    /// // Get the key for the first time.
    /// let session_key = config.take_session_key();
    /// assert_eq!(session_key, Some(key.clone()));
    ///
    /// // Try to get the key again.
    /// let session_key_again = config.take_session_key();
    /// assert_eq!(session_key_again, None);
    /// ```
    #[inline(always)]
    pub fn take_session_key(&self) -> Option<String> {
        let mut key = self.session_key.write().expect("couldn't lock session key");
        key.take()
    }

    /// Returns an iterator over the names and values of all of the extras in
    /// the `self` Config.
    #[inline(always)]
    pub fn extras<'a>(&'a self) -> impl Iterator<Item=(&'a str, &'a Value)> {
        self.extras.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Attempts to retrieve the extra named `name` as a string. If an extra
    /// with that name doesn't exist, returns an `Err` of `NotFound`. If an
    /// extra with that name does exist but is not a string, returns a `BadType`
    /// error.
    pub fn get_str<'a>(&'a self, name: &str) -> config::Result<&'a str> {
        let value = self.extras.get(name).ok_or_else(|| ConfigError::NotFound)?;
        parse!(self, name, value, as_str, "a string")
    }

    /// Attempts to retrieve the extra named `name` as an integer. If an extra
    /// with that name doesn't exist, returns an `Err` of `NotFound`. If an
    /// extra with that name does exist but is not an integer, returns a
    /// `BadType` error.
    pub fn get_int(&self, name: &str) -> config::Result<i64> {
        let value = self.extras.get(name).ok_or_else(|| ConfigError::NotFound)?;
        parse!(self, name, value, as_integer, "an integer")
    }

    /// Attempts to retrieve the extra named `name` as a boolean. If an extra
    /// with that name doesn't exist, returns an `Err` of `NotFound`. If an
    /// extra with that name does exist but is not a boolean, returns a
    /// `BadType` error.
    pub fn get_bool(&self, name: &str) -> config::Result<bool> {
        let value = self.extras.get(name).ok_or_else(|| ConfigError::NotFound)?;
        parse!(self, name, value, as_bool, "a boolean")
    }

    /// Attempts to retrieve the extra named `name` as a float. If an extra
    /// with that name doesn't exist, returns an `Err` of `NotFound`. If an
    /// extra with that name does exist but is not a float, returns a
    /// `BadType` error.
    pub fn get_float(&self, name: &str) -> config::Result<f64> {
        let value = self.extras.get(name).ok_or_else(|| ConfigError::NotFound)?;
        parse!(self, name, value, as_float, "a float")
    }

    /// Returns the path at which the configuration file for `self` is stored.
    /// For instance, if the configuration file is at `/tmp/Rocket.toml`, the
    /// path `/tmp` is returned.
    pub fn root(&self) -> &Path {
        match Path::new(self.filepath.as_str()).parent() {
            Some(parent) => parent,
            None => panic!("root(): filepath {} has no parent", self.filepath)
        }
    }

    /// Sets the `address` in `self` to `var` and returns the structure.
    #[inline(always)]
    pub fn address(mut self, var: String) -> Self {
        self.address = var;
        self
    }

    /// Sets the `port` in `self` to `var` and returns the structure.
    #[inline(always)]
    pub fn port(mut self, var: u16) -> Self {
        self.port = var;
        self
    }

    /// Sets the `log_level` in `self` to `var` and returns the structure.
    #[inline(always)]
    pub fn log_level(mut self, var: LoggingLevel) -> Self {
        self.log_level = var;
        self
    }

    /// Sets the `session_key` in `self` to `var` and returns the structure.
    #[inline(always)]
    pub fn session_key(mut self, var: String) -> Self {
        self.session_key = RwLock::new(Some(var));
        self
    }

    /// Sets the `env` in `self` to `var` and returns the structure.
    #[inline(always)]
    pub fn env(mut self, var: Environment) -> Self {
        self.env = var;
        self
    }

    /// Adds an extra configuration parameter with `name` and `value` to `self`
    /// and returns the structure.
    #[inline(always)]
    pub fn extra(mut self, name: &str, value: &Value) -> Self {
        self.extras.insert(name.into(), value.clone());
        self
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Config[{}] {{ address: {}, port: {}, log_level: {:?} }}",
               self.env, self.address, self.port, self.log_level)
    }
}

impl PartialEq for Config {
    fn eq(&self, other: &Config) -> bool {
        &*self.session_key.read().unwrap() == &*other.session_key.read().unwrap()
            && self.address == other.address
            && self.port == other.port
            && self.log_level == other.log_level
            && self.env == other.env
            && self.extras == other.extras
            && self.filepath == other.filepath
    }
}
