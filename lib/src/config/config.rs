use std::collections::HashMap;
use std::net::ToSocketAddrs;
use std::path::{Path, PathBuf};
use std::convert::AsRef;
use std::fmt;
use std::env;

use super::custom_values::*;
use {num_cpus, base64};
use config::Environment::*;
use config::{Result, ConfigBuilder, Environment, ConfigError, LoggingLevel};
use config::{Table, Value, Array, Datetime};
use http::Key;

/// Structure for Rocket application configuration.
///
/// # Usage
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
///
/// ## General Configuration
///
/// For more information about Rocket's configuration, see the [`config`] module
/// documentaiton.
///
/// [`config`]: /rocket/config/index.html
#[derive(Clone)]
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
    /// The secret key.
    pub(crate) secret_key: SecretKey,
    /// TLS configuration.
    pub(crate) tls: Option<TlsConfig>,
    /// Streaming read size limits.
    pub limits: Limits,
    /// Extra parameters that aren't part of Rocket's core config.
    pub extras: HashMap<String, Value>,
    /// The path to the configuration file this config belongs to.
    pub config_path: PathBuf,
}

macro_rules! config_from_raw {
    ($config:expr, $name:expr, $value:expr,
        $($key:ident => ($type:ident, $set:ident, $map:expr)),+ | _ => $rest:expr) => (
        match $name {
            $(stringify!($key) => {
                super::custom_values::$type($config, $name, $value)
                    .and_then(|parsed| $map($config.$set(parsed)))
            })+
            _ => $rest
        }
    )
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
    pub fn new(env: Environment) -> Result<Config> {
        let cwd = env::current_dir().map_err(|_| ConfigError::BadCWD)?;
        Config::default(env, cwd.as_path().join("Rocket.custom.toml"))
    }

    /// Returns a builder for `Config` structure where the default parameters
    /// are set to those of the development environment. The root configuration
    /// directory is set to the current working directory.
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
    /// let mut my_config = Config::development().unwrap();
    /// my_config.set_port(1001);
    /// ```
    pub fn development() -> Result<Config> {
        Config::new(Environment::Development)
    }

    /// Creates a new configuration using the default parameters from the
    /// staging environment. The root configuration directory is set to the
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
    /// let mut my_config = Config::staging().expect("cwd");
    /// my_config.set_port(1001);
    /// ```
    pub fn staging() -> Result<Config> {
        Config::new(Environment::Staging)
    }

    /// Creates a new configuration using the default parameters from the
    /// production environment. The root configuration directory is set to the
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
    /// let mut my_config = Config::production().expect("cwd");
    /// my_config.set_port(1001);
    /// ```
    pub fn production() -> Result<Config> {
        Config::new(Environment::Production)
    }

    /// Returns the default configuration for the environment `env` given that
    /// the configuration was stored at `config_path`. If `config_path` is not
    /// an absolute path, an `Err` of `ConfigError::BadFilePath` is returned.
    ///
    /// # Panics
    ///
    /// Panics if randomness cannot be retrieved from the OS.
    pub(crate) fn default<P>(env: Environment, path: P) -> Result<Config>
        where P: AsRef<Path>
    {
        let config_path = path.as_ref().to_path_buf();
        if config_path.parent().is_none() {
            return Err(ConfigError::BadFilePath(config_path,
                "Configuration files must be rooted in a directory."));
        }

        // Note: This may truncate if num_cpus::get() / 2 > u16::max. That's okay.
        let default_workers = (num_cpus::get() * 2) as u16;

        // Use a generated secret key by default.
        let key = SecretKey::Generated(Key::generate());

        Ok(match env {
            Development => {
                Config {
                    environment: Development,
                    address: "localhost".to_string(),
                    port: 8000,
                    workers: default_workers,
                    log_level: LoggingLevel::Normal,
                    secret_key: key,
                    tls: None,
                    limits: Limits::default(),
                    extras: HashMap::new(),
                    config_path: config_path,
                }
            }
            Staging => {
                Config {
                    environment: Staging,
                    address: "0.0.0.0".to_string(),
                    port: 8000,
                    workers: default_workers,
                    log_level: LoggingLevel::Normal,
                    secret_key: key,
                    tls: None,
                    limits: Limits::default(),
                    extras: HashMap::new(),
                    config_path: config_path,
                }
            }
            Production => {
                Config {
                    environment: Production,
                    address: "0.0.0.0".to_string(),
                    port: 8000,
                    workers: default_workers,
                    log_level: LoggingLevel::Critical,
                    secret_key: key,
                    tls: None,
                    limits: Limits::default(),
                    extras: HashMap::new(),
                    config_path: config_path,
                }
            }
        })
    }

    /// Constructs a `BadType` error given the entry `name`, the invalid `val`
    /// at that entry, and the `expect`ed type name.
    #[inline(always)]
    pub(crate) fn bad_type(&self,
                           name: &str,
                           actual: &'static str,
                           expect: &'static str) -> ConfigError {
        let id = format!("{}.{}", self.environment, name);
        ConfigError::BadType(id, expect, actual, self.config_path.clone())
    }

    /// Sets the configuration `val` for the `name` entry. If the `name` is one
    /// of "address", "port", "secret_key", "log", or "workers" (the "default"
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
    ///   * **secret_key**: String (192-bit base64)
    ///   * **tls**: Table (`certs` (path as String), `key` (path as String))
    pub(crate) fn set_raw(&mut self, name: &str, val: &Value) -> Result<()> {
        let (id, ok) = (|val| val, |_| Ok(()));
        config_from_raw!(self, name, val,
            address => (str, set_address, id),
            port => (u16, set_port, ok),
            workers => (u16, set_workers, ok),
            secret_key => (str, set_secret_key, id),
            log => (log_level, set_log_level, ok),
            tls => (tls_config, set_raw_tls, id),
            limits => (limits, set_limits, ok)
            | _ => {
                self.extras.insert(name.into(), val.clone());
                Ok(())
            }
        )
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
        let new_path = match self.config_path.file_name() {
            Some(file) => path.as_ref().join(file),
            None => path.as_ref().join("Rocket.custom.toml")
        };

        self.config_path = new_path
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
    pub fn set_address<A: Into<String>>(&mut self, address: A) -> Result<()> {
        let address = address.into();
        if (&*address, 0u16).to_socket_addrs().is_err() {
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
    #[inline]
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
    #[inline]
    pub fn set_workers(&mut self, workers: u16) {
        self.workers = workers;
    }

    /// Sets the `secret_key` in `self` to `key` which must be a 192-bit base64
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
    /// let key = "8Xui8SN4mI+7egV/9dlfYYLGQJeEx4+DwmSQLwDVXJg=";
    /// assert!(config.set_secret_key(key).is_ok());
    /// assert!(config.set_secret_key("hello? anyone there?").is_err());
    /// # Ok(())
    /// # }
    /// ```
    pub fn set_secret_key<K: Into<String>>(&mut self, key: K) -> Result<()> {
        let key = key.into();
        let error = self.bad_type("secret_key", "string",
                                  "a 256-bit base64 encoded string");

        if key.len() != 44 {
            return Err(error);
        }

        let bytes = match base64::decode(&key) {
            Ok(bytes) => bytes,
            Err(_) => return Err(error)
        };

        self.secret_key = SecretKey::Provided(Key::from_master(&bytes));
        Ok(())
    }

    /// Sets the logging level for `self` to `log_level`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::{Config, LoggingLevel, Environment};
    ///
    /// # use rocket::config::ConfigError;
    /// # fn config_test() -> Result<(), ConfigError> {
    /// let mut config = Config::new(Environment::Staging)?;
    /// config.set_log_level(LoggingLevel::Critical);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn set_log_level(&mut self, log_level: LoggingLevel) {
        self.log_level = log_level;
    }

    /// Set the receive limits in `self` to `limits`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::{Config, Limits};
    ///
    /// # use rocket::config::ConfigError;
    /// # fn config_test() -> Result<(), ConfigError> {
    /// let mut config = Config::development()?;
    /// config.set_limits(Limits::default().limit("json", 4 * (1 << 20)));
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn set_limits(&mut self, limits: Limits) {
        self.limits = limits;
    }

    /// Sets the TLS configuration in `self`.
    ///
    /// Certificates are read from `certs_path`. The certificate chain must be
    /// in X.509 PEM format. The private key is read from `key_path`. The
    /// private key must be an RSA key in either PKCS#1 or PKCS#8 PEM format.
    ///
    /// # Errors
    ///
    /// If reading either the certificates or private key fails, an error of
    /// variant `Io` is returned. If either the certificates or private key
    /// files are malformed or cannot be parsed, an error of `BadType` is
    /// returned.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::Config;
    ///
    /// # use rocket::config::ConfigError;
    /// # fn config_test() -> Result<(), ConfigError> {
    /// let mut config = Config::development()?;
    /// config.set_tls("/etc/ssl/my_certs.pem", "/etc/ssl/priv.key")?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "tls")]
    pub fn set_tls(&mut self, certs_path: &str, key_path: &str) -> Result<()> {
        use hyper_sync_rustls::util as tls;
        use hyper_sync_rustls::util::Error::Io;
        let pem_err = "malformed PEM file";

        // Load the certificates.
        let certs = tls::load_certs(self.root_relative(certs_path))
            .map_err(|e| match e {
                Io(e) => ConfigError::Io(e, "tls.certs"),
                _ => self.bad_type("tls", pem_err, "a valid certificates file")
            })?;

        // And now the private key.
        let key = tls::load_private_key(self.root_relative(key_path))
            .map_err(|e| match e {
                Io(e) => ConfigError::Io(e, "tls.key"),
                _ => self.bad_type("tls", pem_err, "a valid private key file")
            })?;

        self.tls = Some(TlsConfig { certs, key });
        Ok(())
    }

    #[doc(hidden)]
    #[cfg(not(feature = "tls"))]
    pub fn set_tls(&mut self, _: &str, _: &str) -> Result<()> {
        self.tls = Some(TlsConfig);
        Ok(())
    }

    #[cfg(not(test))]
    #[inline(always)]
    fn set_raw_tls(&mut self, paths: (&str, &str)) -> Result<()> {
        self.set_tls(paths.0, paths.1)
    }

    #[cfg(test)]
    fn set_raw_tls(&mut self, _: (&str, &str)) -> Result<()> {
        Ok(())
    }

    /// Sets the extras for `self` to be the key/value pairs in `extras`.
    /// encoded string.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::collections::HashMap;
    /// use rocket::config::{Config, Environment};
    ///
    /// # use rocket::config::ConfigError;
    /// # fn config_test() -> Result<(), ConfigError> {
    /// let mut config = Config::new(Environment::Staging)?;
    ///
    /// // Create the `extras` map.
    /// let mut extras = HashMap::new();
    /// extras.insert("another_port".to_string(), 1044.into());
    /// extras.insert("templates".to_string(), "my_dir".into());
    ///
    /// config.set_extras(extras);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
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
    /// use rocket::config::{Config, Environment};
    ///
    /// # use rocket::config::ConfigError;
    /// # fn config_test() -> Result<(), ConfigError> {
    /// let mut config = Config::new(Environment::Staging)?;
    /// assert_eq!(config.extras().count(), 0);
    ///
    /// // Add a couple of extras to the config.
    /// let mut extras = HashMap::new();
    /// extras.insert("another_port".to_string(), 1044.into());
    /// extras.insert("templates".to_string(), "my_dir".into());
    /// config.set_extras(extras);
    ///
    /// assert_eq!(config.extras().count(), 2);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn extras<'a>(&'a self) -> impl Iterator<Item=(&'a str, &'a Value)> {
        self.extras.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Retrieves the secret key from `self`.
    #[inline]
    pub(crate) fn secret_key(&self) -> &Key {
        self.secret_key.inner()
    }

    /// Attempts to retrieve the extra named `name` as a borrowed string.
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
    pub fn get_str<'a>(&'a self, name: &str) -> Result<&'a str> {
        let val = self.extras.get(name).ok_or_else(|| ConfigError::NotFound)?;
        val.as_str().ok_or_else(|| self.bad_type(name, val.type_str(), "a string"))
    }

    /// Attempts to retrieve the extra named `name` as an owned string.
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
    /// assert_eq!(config.get_string("my_extra"), Ok("extra_value".to_string()));
    /// ```
    pub fn get_string(&self, name: &str) -> Result<String> {
        self.get_str(name).map(|s| s.to_string())
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
    pub fn get_int(&self, name: &str) -> Result<i64> {
        let val = self.extras.get(name).ok_or_else(|| ConfigError::NotFound)?;
        val.as_integer().ok_or_else(|| self.bad_type(name, val.type_str(), "an integer"))
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
    pub fn get_bool(&self, name: &str) -> Result<bool> {
        let val = self.extras.get(name).ok_or_else(|| ConfigError::NotFound)?;
        val.as_bool().ok_or_else(|| self.bad_type(name, val.type_str(), "a boolean"))
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
    pub fn get_float(&self, name: &str) -> Result<f64> {
        let val = self.extras.get(name).ok_or_else(|| ConfigError::NotFound)?;
        val.as_float().ok_or_else(|| self.bad_type(name, val.type_str(), "a float"))
    }

    /// Attempts to retrieve the extra named `name` as a slice of an array.
    ///
    /// # Errors
    ///
    /// If an extra with `name` doesn't exist, returns an `Err` of `NotFound`.
    /// If an extra with `name` _does_ exist but is not an array, returns a
    /// `BadType` error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::{Config, Environment};
    ///
    /// let config = Config::build(Environment::Staging)
    ///     .extra("numbers", vec![1, 2, 3])
    ///     .unwrap();
    ///
    /// assert!(config.get_slice("numbers").is_ok());
    /// ```
    pub fn get_slice(&self, name: &str) -> Result<&Array> {
        let val = self.extras.get(name).ok_or_else(|| ConfigError::NotFound)?;
        val.as_array().ok_or_else(|| self.bad_type(name, val.type_str(), "an array"))
    }

    /// Attempts to retrieve the extra named `name` as a table.
    ///
    /// # Errors
    ///
    /// If an extra with `name` doesn't exist, returns an `Err` of `NotFound`.
    /// If an extra with `name` _does_ exist but is not a table, returns a
    /// `BadType` error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::collections::BTreeMap;
    /// use rocket::config::{Config, Environment};
    ///
    /// let mut table = BTreeMap::new();
    /// table.insert("my_value".to_string(), 1);
    ///
    /// let config = Config::build(Environment::Staging)
    ///     .extra("my_table", table)
    ///     .unwrap();
    ///
    /// assert!(config.get_table("my_table").is_ok());
    /// ```
    pub fn get_table(&self, name: &str) -> Result<&Table> {
        let val = self.extras.get(name).ok_or_else(|| ConfigError::NotFound)?;
        val.as_table().ok_or_else(|| self.bad_type(name, val.type_str(), "a table"))
    }

    /// Attempts to retrieve the extra named `name` as a datetime value.
    ///
    /// # Errors
    ///
    /// If an extra with `name` doesn't exist, returns an `Err` of `NotFound`.
    /// If an extra with `name` _does_ exist but is not a datetime, returns a
    /// `BadType` error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::{Config, Environment, Value, Datetime};
    ///
    /// let date = "1979-05-27T00:32:00-07:00".parse::<Datetime>().unwrap();
    ///
    /// let config = Config::build(Environment::Staging)
    ///     .extra("my_date", Value::Datetime(date.clone()))
    ///     .unwrap();
    ///
    /// assert_eq!(config.get_datetime("my_date"), Ok(&date));
    /// ```
    pub fn get_datetime(&self, name: &str) -> Result<&Datetime> {
        let v = self.extras.get(name).ok_or_else(|| ConfigError::NotFound)?;
        v.as_datetime().ok_or_else(|| self.bad_type(name, v.type_str(), "a datetime"))
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

    /// If `path` is a relative path, `path` is appended to the [`root`] at
    /// which the configuration file for `self` is stored and the new path is
    /// returned. If `path` is absolute, `path` is returned unaltered.
    ///
    /// [`root`]: /rocket/struct.Config.html#method.root
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::env::current_dir;
    /// use std::path::Path;
    /// use rocket::config::{Config, Environment};
    ///
    /// let config = Config::new(Environment::Staging)
    ///     .expect("can retrieve current directory");
    ///
    /// assert_eq!(config.root(), current_dir().unwrap());
    /// assert_eq!(config.root_relative("abc"), config.root().join("abc"));
    /// assert_eq!(config.root_relative("/abc"), Path::new("/abc"));
    /// ```
    pub fn root_relative<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        let path = path.as_ref();
        if path.is_absolute() {
            path.into()
        } else {
            self.root().join(path)
        }
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = f.debug_struct("Config");
        s.field("environment", &self.environment);
        s.field("address", &self.address);
        s.field("port", &self.port);
        s.field("workers", &self.workers);
        s.field("log_level", &self.log_level);

        for (key, value) in self.extras() {
            s.field(key, &value);
        }

        s.finish()
    }
}

/// Doesn't consider the secret key or config path.
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
