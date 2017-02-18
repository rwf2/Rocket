use std::collections::HashMap;
use std::net::{IpAddr, lookup_host};
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::convert::AsRef;
use std::fmt;
use std::env;

use config::Environment::*;
use config::{self, Value, ConfigBuilder, Environment, ConfigError};

use num_cpus;
use logger::LoggingLevel;

/// Structure for Rocket application configuration.
///
/// A `Config` structure is typically built using the [build](#method.build)
/// method and [ConfigBuilder](/rocket/config/struct.ConfigBuilder.html)
/// methods:
///
/// ```rust
/// use rocket::config::{Config, Environment};
///
/// # #[allow(unused_variables)]
/// let config = Config::build(Environment::Staging)
///     .address("127.0.0.1")
///     .port(700)
///     .workers(12)
///     .unwrap();
/// ```
pub struct Config {
    /// The environment that this configuration corresponds to.
    pub environment: Environment,
    /// The address to serve on.
    pub address: String,
    /// The port to serve on.
    pub port: u16,
    /// The number of workers to run concurrently.
    pub workers: u16,
    /// How much information to log.
    pub log_level: LoggingLevel,
    /// Extra parameters that aren't part of Rocket's core config.
    pub extras: HashMap<String, Value>,
    /// The path to the configuration file this config belongs to.
    pub config_path: PathBuf,
    /// The session key.
    session_key: RwLock<Option<String>>,
}

macro_rules! parse {
    ($conf:expr, $name:expr, $val:expr, $method:ident, $expect: expr) => (
        $val.$method().ok_or_else(|| {
            $conf.bad_type($name, $val.type_str(), $expect)
        })
    );
}

impl Config {
    /// Returns a builder for `Config` structure where the default parameters
    /// are set to those of `env`. The root configuration directory is set to
    /// the current working directory.
    ///
    /// # Panics
    ///
    /// Panics if the current directory cannot be retrieved.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::{Config, Environment};
    ///
    /// # #[allow(unused_variables)]
    /// let config = Config::build(Environment::Staging)
    ///     .address("127.0.0.1")
    ///     .port(700)
    ///     .workers(12)
    ///     .unwrap();
    /// ```
    pub fn build(env: Environment) -> ConfigBuilder {
        ConfigBuilder::new(env)
    }

    /// Creates a new configuration using the default parameters for the
    /// environment `env`. The root configuration directory is set to the
    /// current working directory.
    ///
    /// # Errors
    ///
    /// If the current directory cannot be retrieved, a `BadCWD` error is
    /// returned.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::{Config, Environment};
    ///
    /// let mut my_config = Config::new(Environment::Production).expect("cwd");
    /// my_config.set_port(1001);
    /// ```
    pub fn new(env: Environment) -> config::Result<Config> {
        let cwd = env::current_dir().map_err(|_| ConfigError::BadCWD)?;
        Config::default(env, cwd.as_path().join("Rocket.custom.toml"))
    }

    // Aliases to `default_for` before the method is removed.
    pub(crate) fn default<P>(env: Environment, path: P) -> config::Result<Config>
        where P: AsRef<Path>
    {
        #[allow(deprecated)]
        Config::default_for(env, path)
    }

    /// Returns the default configuration for the environment `env` given that
    /// the configuration was stored at `config_path`. If `config_path` is not
    /// an absolute path, an `Err` of `ConfigError::BadFilePath` is returned.
    #[deprecated(since="0.2", note="use the `new` or `build` methods instead")]
    pub fn default_for<P>(env: Environment, config_path: P) -> config::Result<Config>
        where P: AsRef<Path>
    {
        let config_path = config_path.as_ref().to_path_buf();
        if config_path.parent().is_none() {
            return Err(ConfigError::BadFilePath(config_path,
                "Configuration files must be rooted in a directory."));
        }

        // Note: This may truncate if num_cpus::get() > u16::max. That's okay.
        let default_workers = ::std::cmp::max(num_cpus::get(), 2) as u16;

        Ok(match env {
            Development => {
                Config {
                    environment: Development,
                    address: "localhost".to_string(),
                    port: 8000,
                    workers: default_workers,
                    log_level: LoggingLevel::Normal,
                    session_key: RwLock::new(None),
                    extras: HashMap::new(),
                    config_path: config_path,
                }
            }
            Staging => {
                Config {
                    environment: Staging,
                    address: "0.0.0.0".to_string(),
                    port: 80,
                    workers: default_workers,
                    log_level: LoggingLevel::Normal,
                    session_key: RwLock::new(None),
                    extras: HashMap::new(),
                    config_path: config_path,
                }
            }
            Production => {
                Config {
                    environment: Production,
                    address: "0.0.0.0".to_string(),
                    port: 80,
                    workers: default_workers,
                    log_level: LoggingLevel::Critical,
                    session_key: RwLock::new(None),
                    extras: HashMap::new(),
                    config_path: config_path,
                }
            }
        })
    }

    /// Constructs a `BadType` error given the entry `name`, the invalid `val`
    /// at that entry, and the `expect`ed type name.
    #[inline(always)]
    fn bad_type(&self, name: &str, actual: &'static str, expect: &'static str)
        -> ConfigError {
        let id = format!("{}.{}", self.environment, name);
        ConfigError::BadType(id, expect, actual, self.config_path.clone())
    }

    // Aliases to `set` before the method is removed.
    pub(crate) fn set_raw(&mut self, name: &str, val: &Value) -> config::Result<()> {
        #[allow(deprecated)]
        self.set(name, val)
    }

    /// Sets the configuration `val` for the `name` entry. If the `name` is one
    /// of "address", "port", "session_key", "log", or "workers" (the "default"
    /// values), the appropriate value in the `self` Config structure is set.
    /// Otherwise, the value is stored as an `extra`.
    ///
    /// For each of the default values, the following `Value` variant is
    /// expected. If a different variant is supplied, a `BadType` `Err` is
    /// returned:
    ///
    ///   * **address**: String
    ///   * **port**: Integer (16-bit unsigned)
    ///   * **workers**: Integer (16-bit unsigned)
    ///   * **log**: String
    ///   * **session_key**: String (192-bit base64)
    #[deprecated(since="0.2", note="use the set_{param} methods instead")]
    pub fn set(&mut self, name: &str, val: &Value) -> config::Result<()> {
        if name == "address" {
            let address_str = parse!(self, name, val, as_str, "a string")?;
            self.set_address(address_str)?;
        } else if name == "port" {
            let port = parse!(self, name, val, as_integer, "an integer")?;
            if port < 0 || port > (u16::max_value() as i64) {
                return Err(self.bad_type(name, val.type_str(), "a 16-bit unsigned integer"))
            }

            self.set_port(port as u16);
        } else if name == "workers" {
            let workers = parse!(self, name, val, as_integer, "an integer")?;
            if workers < 0 || workers > (u16::max_value() as i64) {
                return Err(self.bad_type(name, val.type_str(), "a 16-bit unsigned integer"));
            }

            self.set_workers(workers as u16);
        } else if name == "session_key" {
            let key = parse!(self, name, val, as_str, "a string")?;
            self.set_session_key(key)?;
        } else if name == "log" {
            let level_str = parse!(self, name, val, as_str, "a string")?;
            let expect = "log level ('normal', 'critical', 'debug')";
            match level_str.parse() {
                Ok(level) => self.set_log_level(level),
                Err(_) => return Err(self.bad_type(name, val.type_str(), expect))
            }
        } else {
            self.extras.insert(name.into(), val.clone());
        }

        Ok(())
    }

    /// Sets the root directory of this configuration to `root`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::path::Path;
    /// use rocket::config::{Config, Environment};
    ///
    /// # use rocket::config::ConfigError;
    /// # fn config_test() -> Result<(), ConfigError> {
    /// let mut config = Config::new(Environment::Staging)?;
    /// config.set_root("/tmp/my_app");
    ///
    /// assert_eq!(config.root(), Path::new("/tmp/my_app"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn set_root<P: AsRef<Path>>(&mut self, path: P) {
        self.config_path = path.as_ref().join("Rocket.custom.toml")
    }

    /// Sets the address of `self` to `address`.
    ///
    /// # Errors
    ///
    /// If `address` is not a valid IP address or hostname, returns a `BadType`
    /// error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::{Config, Environment};
    ///
    /// # use rocket::config::ConfigError;
    /// # fn config_test() -> Result<(), ConfigError> {
    /// let mut config = Config::new(Environment::Staging)?;
    /// assert!(config.set_address("localhost").is_ok());
    /// assert!(config.set_address("::").is_ok());
    /// assert!(config.set_address("?").is_err());
    /// # Ok(())
    /// # }
    /// ```
    pub fn set_address<A: Into<String>>(&mut self, address: A) -> config::Result<()> {
        let address = address.into();
        if address.parse::<IpAddr>().is_err() && lookup_host(&address).is_err() {
            return Err(self.bad_type("address", "string", "a valid hostname or IP"));
        }

        self.address = address;
        Ok(())
    }

    /// Sets the `port` of `self` to `port`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::{Config, Environment};
    ///
    /// # use rocket::config::ConfigError;
    /// # fn config_test() -> Result<(), ConfigError> {
    /// let mut config = Config::new(Environment::Staging)?;
    /// config.set_port(1024);
    /// # Ok(())
    /// # }
    /// ```
    pub fn set_port(&mut self, port: u16) {
        self.port = port;
    }

    /// Sets the number of `workers` in `self` to `workers`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::{Config, Environment};
    ///
    /// # use rocket::config::ConfigError;
    /// # fn config_test() -> Result<(), ConfigError> {
    /// let mut config = Config::new(Environment::Staging)?;
    /// config.set_workers(64);
    /// # Ok(())
    /// # }
    /// ```
    pub fn set_workers(&mut self, workers: u16) {
        self.workers = workers;
    }

    /// Sets the `session_key` in `self` to `key` which must be a 192-bit base64
    /// encoded string.
    ///
    /// # Errors
    ///
    /// If `key` is not a valid 192-bit base64 encoded string, returns a
    /// `BadType` error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::{Config, Environment};
    ///
    /// # use rocket::config::ConfigError;
    /// # fn config_test() -> Result<(), ConfigError> {
    /// let mut config = Config::new(Environment::Staging)?;
    /// assert!(config.set_session_key("VheMwXIBygSmOlZAhuWl2B+zgvTN3WW5").is_ok());
    /// assert!(config.set_session_key("hello? anyone there?").is_err());
    /// # Ok(())
    /// # }
    /// ```
    pub fn set_session_key<K: Into<String>>(&mut self, key: K) -> config::Result<()> {
        let key = key.into();
        if key.len() != 32 {
            return Err(self.bad_type("session_key",
                                     "string",
                                     "a 192-bit base64 string"));
        }

        self.session_key = RwLock::new(Some(key));
        Ok(())
    }

    /// Sets the logging level for `self` to `log_level`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::LoggingLevel;
    /// use rocket::config::{Config, Environment};
    ///
    /// # use rocket::config::ConfigError;
    /// # fn config_test() -> Result<(), ConfigError> {
    /// let mut config = Config::new(Environment::Staging)?;
    /// config.set_log_level(LoggingLevel::Critical);
    /// # Ok(())
    /// # }
    /// ```
    pub fn set_log_level(&mut self, log_level: LoggingLevel) {
        self.log_level = log_level;
    }

    /// Sets the extras for `self` to be the key/value pairs in `extras`.
    /// encoded string.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::collections::HashMap;
    /// use rocket::config::{Config, Environment, IntoValue};
    ///
    /// # use rocket::config::ConfigError;
    /// # fn config_test() -> Result<(), ConfigError> {
    /// let mut config = Config::new(Environment::Staging)?;
    ///
    /// // Create the `extras` map.
    /// let mut extras = HashMap::new();
    /// extras.insert("another_port".to_string(), 1044.into_value());
    /// extras.insert("templates".to_string(), "my_dir".into_value());
    ///
    /// config.set_extras(extras);
    /// # Ok(())
    /// # }
    /// ```
    pub fn set_extras(&mut self, extras: HashMap<String, Value>) {
        self.extras = extras;
    }

    /// Returns an iterator over the names and values of all of the extras in
    /// `self`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::collections::HashMap;
    /// use rocket::config::{Config, Environment, IntoValue};
    ///
    /// # use rocket::config::ConfigError;
    /// # fn config_test() -> Result<(), ConfigError> {
    /// let mut config = Config::new(Environment::Staging)?;
    /// assert_eq!(config.extras().count(), 0);
    ///
    /// // Add a couple of extras to the config.
    /// let mut extras = HashMap::new();
    /// extras.insert("another_port".to_string(), 1044.into_value());
    /// extras.insert("templates".to_string(), "my_dir".into_value());
    /// config.set_extras(extras);
    ///
    /// assert_eq!(config.extras().count(), 2);
    /// # Ok(())
    /// # }
    /// ```
    pub fn extras<'a>(&'a self) -> impl Iterator<Item=(&'a str, &'a Value)> {
        self.extras.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Moves the session key string out of the `self` Config, if there is one.
    /// Because the value is moved out, subsequent calls will result in a return
    /// value of `None`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::{Config, Environment};
    ///
    /// // Create a new config with a session key.
    /// let key = "adL5fFIPmZBrlyHk2YT4NLV3YCk2gFXz";
    /// let config = Config::build(Environment::Staging)
    ///     .session_key(key)
    ///     .unwrap();
    ///
    /// // Get the key for the first time.
    /// let session_key = config.take_session_key();
    /// assert_eq!(session_key, Some(key.to_string()));
    ///
    /// // Try to get the key again.
    /// let session_key_again = config.take_session_key();
    /// assert_eq!(session_key_again, None);
    /// ```
    #[inline]
    pub fn take_session_key(&self) -> Option<String> {
        let mut key = self.session_key.write().expect("couldn't lock session key");
        key.take()
    }

    /// Attempts to retrieve the extra named `name` as a string.
    ///
    /// # Errors
    ///
    /// If an extra with `name` doesn't exist, returns an `Err` of `NotFound`.
    /// If an extra with `name` _does_ exist but is not a string, returns a
    /// `BadType` error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::{Config, Environment};
    ///
    /// let config = Config::build(Environment::Staging)
    ///     .extra("my_extra", "extra_value")
    ///     .unwrap();
    ///
    /// assert_eq!(config.get_str("my_extra"), Ok("extra_value"));
    /// ```
    pub fn get_str<'a>(&'a self, name: &str) -> config::Result<&'a str> {
        let value = self.extras.get(name).ok_or_else(|| ConfigError::NotFound)?;
        parse!(self, name, value, as_str, "a string")
    }

    /// Attempts to retrieve the extra named `name` as an integer.
    ///
    /// # Errors
    ///
    /// If an extra with `name` doesn't exist, returns an `Err` of `NotFound`.
    /// If an extra with `name` _does_ exist but is not an integer, returns a
    /// `BadType` error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::{Config, Environment};
    ///
    /// let config = Config::build(Environment::Staging)
    ///     .extra("my_extra", 1025)
    ///     .unwrap();
    ///
    /// assert_eq!(config.get_int("my_extra"), Ok(1025));
    /// ```
    pub fn get_int(&self, name: &str) -> config::Result<i64> {
        let value = self.extras.get(name).ok_or_else(|| ConfigError::NotFound)?;
        parse!(self, name, value, as_integer, "an integer")
    }

    /// Attempts to retrieve the extra named `name` as a boolean.
    ///
    /// # Errors
    ///
    /// If an extra with `name` doesn't exist, returns an `Err` of `NotFound`.
    /// If an extra with `name` _does_ exist but is not a boolean, returns a
    /// `BadType` error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::{Config, Environment};
    ///
    /// let config = Config::build(Environment::Staging)
    ///     .extra("my_extra", true)
    ///     .unwrap();
    ///
    /// assert_eq!(config.get_bool("my_extra"), Ok(true));
    /// ```
    pub fn get_bool(&self, name: &str) -> config::Result<bool> {
        let value = self.extras.get(name).ok_or_else(|| ConfigError::NotFound)?;
        parse!(self, name, value, as_bool, "a boolean")
    }

    /// Attempts to retrieve the extra named `name` as a float.
    ///
    /// # Errors
    ///
    /// If an extra with `name` doesn't exist, returns an `Err` of `NotFound`.
    /// If an extra with `name` _does_ exist but is not a float, returns a
    /// `BadType` error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::{Config, Environment};
    ///
    /// let config = Config::build(Environment::Staging)
    ///     .extra("pi", 3.14159)
    ///     .unwrap();
    ///
    /// assert_eq!(config.get_float("pi"), Ok(3.14159));
    /// ```
    pub fn get_float(&self, name: &str) -> config::Result<f64> {
        let value = self.extras.get(name).ok_or_else(|| ConfigError::NotFound)?;
        parse!(self, name, value, as_float, "a float")
    }

    /// Returns the path at which the configuration file for `self` is stored.
    /// For instance, if the configuration file is at `/tmp/Rocket.toml`, the
    /// path `/tmp` is returned.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::env::current_dir;
    /// use rocket::config::{Config, Environment};
    ///
    /// let config = Config::new(Environment::Staging)
    ///     .expect("can retrieve current directory");
    ///
    /// assert_eq!(config.root(), current_dir().unwrap());
    /// ```
    pub fn root(&self) -> &Path {
        match self.config_path.parent() {
            Some(parent) => parent,
            None => panic!("root(): path {:?} has no parent", self.config_path)
        }
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Config[{}] {{ address: {}, port: {}, workers: {}, log: {:?}",
               self.environment, self.address, self.port, self.workers, self.log_level)?;

        for (key, value) in self.extras() {
            write!(f, ", {}: {}", key, value)?;
        }

        write!(f, " }}")
    }
}

/// Doesn't consider the session key or config path.
impl PartialEq for Config {
    fn eq(&self, other: &Config) -> bool {
        self.address == other.address
            && self.port == other.port
            && self.workers == other.workers
            && self.log_level == other.log_level
            && self.environment == other.environment
            && self.extras == other.extras
    }
}
