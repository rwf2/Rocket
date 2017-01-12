use std::collections::{HashMap, BTreeMap};
use std::hash::Hash;

use config::{Result, Config, Value, Environment};
use logger::LoggingLevel;

/// The core configuration structure.
#[derive(Clone)]
pub struct ConfigBuilder {
    /// The environment that this configuration corresponds to.
    pub environment: Environment,
    /// The address to serve on.
    pub address: String,
    /// The port to serve on.
    pub port: u16,
    /// The number of workers to run in parallel.
    pub workers: u16,
    /// How much information to log.
    pub log_level: LoggingLevel,
    /// The session key.
    pub session_key: Option<String>,
    /// Any extra parameters that aren't part of Rocket's config.
    pub extras: HashMap<String, Value>,
}

impl ConfigBuilder {
    pub fn new(environment: Environment) -> ConfigBuilder {
        let config = Config::new(environment)
            .expect("ConfigBuilder::new(): couldn't get current directory.");

        ConfigBuilder {
            environment: config.environment,
            address: config.address,
            port: config.port,
            workers: config.workers,
            log_level: config.log_level,
            session_key: None,
            extras: config.extras,
        }
    }

    /// Sets the `address` in `self` to `address` and returns the structure.
    #[inline(always)]
    pub fn address<A: Into<String>>(mut self, address: A) -> Self {
        self.address = address.into();
        self
    }

    /// Sets the `port` in `self` to `port` and returns the structure.
    #[inline(always)]
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Sets the `workers` in `self` to `workers` and returns the structure.
    #[inline(always)]
    pub fn workers(mut self, workers: u16) -> Self {
        self.workers = workers;
        self
    }

    /// Sets the `log_level` in `self` to `log_level` and returns the structure.
    #[inline(always)]
    pub fn log_level(mut self, log_level: LoggingLevel) -> Self {
        self.log_level = log_level;
        self
    }

    /// Sets the `session_key` in `self` to `key` and returns the structure.
    #[inline(always)]
    pub fn session_key<K: Into<String>>(mut self, key: K) -> Self {
        self.session_key = Some(key.into());
        self
    }

    /// Sets the environment in `self` to `env` and returns the structure.
    #[inline(always)]
    pub fn environment(mut self, env: Environment) -> Self {
        self.environment = env;
        self
    }

    /// Adds an extra configuration parameter with `name` and `value` to `value`
    /// and returns the structure. The value can be any type that implements the
    /// `IntoValue` trait defined in this module.
    #[inline(always)]
    pub fn extra<V: IntoValue>(mut self, name: &str, value: V) -> Self {
        self.extras.insert(name.into(), value.into_value());
        self
    }

    // TODO: Collect all errors into one `Several` `ConfigError` variant.
    pub fn finalize(self) -> Result<Config> {
        let mut config = Config::new(self.environment)?;
        config.set_address(self.address)?;
        config.set_port(self.port);
        config.set_workers(self.workers);
        config.set_log_level(self.log_level);
        config.set_extras(self.extras);

        if let Some(key) = self.session_key {
            config.set_session_key(key)?;
        }

        Ok(config)
    }

    #[inline(always)]
    pub fn unwrap(self) -> Config {
        self.finalize().expect("ConfigBuilder::unwrap() failed")
    }
}

pub trait IntoValue {
    fn into_value(self) -> Value;
}

impl<'a> IntoValue for &'a str {
    fn into_value(self) -> Value {
        Value::String(self.to_string())
    }
}

impl IntoValue for Value {
    fn into_value(self) -> Value {
        self
    }
}

impl<V: IntoValue> IntoValue for Vec<V> {
    fn into_value(self) -> Value {
        Value::Array(self.into_iter().map(|v| v.into_value()).collect())
    }
}

impl<S: Into<String>, V: IntoValue> IntoValue for BTreeMap<S, V> {
    fn into_value(self) -> Value {
        let table = self.into_iter()
            .map(|(s, v)| (s.into(), v.into_value()))
            .collect();

        Value::Table(table)
    }
}

impl<S: Into<String> + Hash + Eq, V: IntoValue> IntoValue for HashMap<S, V> {
    fn into_value(self) -> Value {
        let table = self.into_iter()
            .map(|(s, v)| (s.into(), v.into_value()))
            .collect();

        Value::Table(table)
    }
}

macro_rules! impl_into_value {
    ($variant:ident : $t:ty) => ( impl_into_value!($variant: $t,); );

    ($variant:ident : $t:ty, $($extra:tt)*) => (
        impl IntoValue for $t {
            fn into_value(self) -> Value {
                Value::$variant(self $($extra)*)
            }
        }
    )
}

impl_into_value!(String: String);
impl_into_value!(Integer: i64);
impl_into_value!(Integer: isize, as i64);
impl_into_value!(Integer: i32, as i64);
impl_into_value!(Integer: i8, as i64);
impl_into_value!(Integer: u8, as i64);
impl_into_value!(Integer: u32, as i64);
impl_into_value!(Boolean: bool);
impl_into_value!(Float: f64);
impl_into_value!(Float: f32, as f64);

