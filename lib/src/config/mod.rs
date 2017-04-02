//! Application configuration and configuration parameter retrieval.
//!
//! This module implements configuration handling for Rocket. It implements the
//! parsing and interpretation of the `Rocket.toml` config file and
//! `ROCKET_{PARAM}` environment variables. It also allows libraries to access
//! values that have been configured by the user.
//!
//! ## Application Configuration
//!
//! ### Environments
//!
//! Rocket applications are always running in one of three environments:
//!
//!   * development _or_ dev
//!   * staging _or_ stage
//!   * production _or_ prod
//!
//! Each environment can contain different configuration parameters. By default,
//! Rocket applications run in the **development** environment. The environment
//! can be changed via the `ROCKET_ENV` environment variable. For example, to
//! start a Rocket application in the **production** environment:
//!
//! ```sh
//! ROCKET_ENV=production ./target/release/rocket_app
//! ```
//!
//! ### Configuration Parameters
//!
//! Each environments consists of several standard configuration parameters as
//! well as an arbitrary number of _extra_ configuration parameters, which are
//! not used by Rocket itself but can be used by external libraries. The
//! standard configuration parameters are:
//!
//!   * **address**: _[string]_ an IP address or host the application will
//!     listen on
//!     * examples: `"localhost"`, `"0.0.0.0"`, `"1.2.3.4"`
//!   * **port**: _[integer]_ a port number to listen on
//!     * examples: `"8000"`, `"80"`, `"4242"`
//!   * **workers**: _[integer]_ the number of concurrent workers to use
//!     * examples: `12`, `1`, `4`
//!   * **log**: _[string]_ how much information to log; one of `"normal"`,
//!     `"debug"`, or `"critical"`
//!   * **session_key**: _[string]_ a 256-bit base64 encoded string (44
//!     characters) to use as the session key
//!     * example: `"8Xui8SN4mI+7egV/9dlfYYLGQJeEx4+DwmSQLwDVXJg="`
//!
//! ### Rocket.toml
//!
//! The `Rocket.toml` file is used to specify the configuration parameters for
//! each environment. The file is optional. If it is not present, the default
//! configuration parameters are used.
//!
//! The file must be a series of TOML tables, at most one for each environment
//! and an optional "global" table, where each table contains key-value pairs
//! corresponding to configuration parameters for that environment. If a
//! configuration parameter is missing, the default value is used. The following
//! is a complete `Rocket.toml` file, where every standard configuration
//! parameter is specified with the default value:
//!
//! ```toml
//! [development]
//! address = "localhost"
//! port = 8000
//! workers = max(number_of_cpus, 2)
//! log = "normal"
//! session_key = [randomly generated at launch]
//!
//! [staging]
//! address = "0.0.0.0"
//! port = 80
//! workers = max(number_of_cpus, 2)
//! log = "normal"
//! session_key = [randomly generated at launch]
//!
//! [production]
//! address = "0.0.0.0"
//! port = 80
//! workers = max(number_of_cpus, 2)
//! log = "critical"
//! session_key = [randomly generated at launch]
//! ```
//!
//! The `workers` and `session_key` default parameters are computed by Rocket
//! automatically; the values above are not valid TOML syntax. When manually
//! specifying the number of workers, the value should be an integer: `workers =
//! 10`. When manually specifying the session key, the value should a 256-bit
//! base64 encoded string. Such a string can be generated with the `openssl`
//! command line tool: `openssl rand -base64 32`.
//!
//! The "global" pseudo-environment can be used to set and/or override
//! configuration parameters globally. A parameter defined in a `[global]` table
//! sets, or overrides if already present, that parameter in every environment.
//! For example, given the following `Rocket.toml` file, the value of `address`
//! will be `"1.2.3.4"` in every environment:
//!
//! ```toml
//! [global]
//! address = "1.2.3.4"
//!
//! [development]
//! address = "localhost"
//!
//! [production]
//! address = "0.0.0.0"
//! ```
//!
//! ### Environment Variables
//!
//! All configuration parameters, including extras, can be overridden through
//! environment variables. To override the configuration parameter `{param}`,
//! use an environment variable named `ROCKET_{PARAM}`. For instance, to
//! override the "port" configuration parameter, you can run your application
//! with:
//!
//! ```sh
//! ROCKET_PORT=3721 ./your_application
//! ```
//!
//! Environment variables take precedence over all other configuration methods:
//! if the variable is set, it will be used as the value for the parameter.
//!
//! ## Retrieving Configuration Parameters
//!
//! Configuration parameters for the currently active configuration environment
//! can be retrieved via the [active](fn.active.html) function and methods on
//! the [Config](struct.Config.html) structure. The general structure is to call
//! `active` and then one of the `get_` methods on the returned `Config`
//! structure.
//!
//! As an example, consider the following code used by the `Template` type to
//! retrieve the value of the `template_dir` configuration parameter. If the
//! value isn't present or isn't a string, a default value is used.
//!
//! ```rust
//! use std::path::PathBuf;
//! use rocket::config::{self, ConfigError};
//!
//! const DEFAULT_TEMPLATE_DIR: &'static str = "templates";
//!
//! # #[allow(unused_variables)]
//! let template_dir = config::active().ok_or(ConfigError::NotFound)
//!     .map(|config| config.root().join(DEFAULT_TEMPLATE_DIR))
//!     .unwrap_or_else(|_| PathBuf::from(DEFAULT_TEMPLATE_DIR));
//! ```
//!
//! Libraries should always use a default if a parameter is not defined.

mod error;
mod environment;
mod config;
mod builder;
mod toml_ext;

use std::sync::{Once, ONCE_INIT};
use std::fs::{self, File};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process;
use std::env;

use toml;

pub use toml::{Array, Table, Value};
pub use self::error::{ConfigError, ParsingError};
pub use self::environment::Environment;
pub use self::config::Config;
pub use self::builder::ConfigBuilder;
pub use self::toml_ext::IntoValue;

use self::Environment::*;
use self::environment::CONFIG_ENV;
use self::toml_ext::parse_simple_toml_value;
use logger::{self, LoggingLevel};
use http::uncased::uncased_eq;

static INIT: Once = ONCE_INIT;
static mut CONFIG: Option<RocketConfig> = None;

const CONFIG_FILENAME: &'static str = "Rocket.toml";
const GLOBAL_ENV_NAME: &'static str = "global";
const ENV_VAR_PREFIX: &'static str = "ROCKET_";
const PREHANDLED_VARS: [&'static str; 2] = ["ROCKET_CODEGEN_DEBUG", CONFIG_ENV];

/// Wraps `std::result` with the error type of
/// [ConfigError](enum.ConfigError.html).
pub type Result<T> = ::std::result::Result<T, ConfigError>;

#[doc(hidden)]
#[derive(Debug, PartialEq)]
pub struct RocketConfig {
    pub active_env: Environment,
    config: HashMap<Environment, Config>,
}

impl RocketConfig {
    /// Create a new configuration using the passed in `config` for all
    /// environments. The Rocket.toml file is ignored, as are environment
    /// variables.
    ///
    /// # Panics
    ///
    /// If the current working directory can't be retrieved, this function
    /// panics.
    pub fn new(config: Config) -> RocketConfig {
        let f = config.config_path.clone();
        let active_env = config.environment;

        // None of these unwraps should fail since the filename is coming from
        // an existing connfig.
        let mut configs = HashMap::new();
        configs.insert(Development, Config::default(Development, &f).unwrap());
        configs.insert(Staging, Config::default(Staging, &f).unwrap());
        configs.insert(Production, Config::default(Production, &f).unwrap());
        configs.insert(active_env, config);

        RocketConfig {
            active_env: active_env,
            config: configs
        }
    }

    /// Read the configuration from the `Rocket.toml` file. The file is search
    /// for recursively up the tree, starting from the CWD.
    pub fn read() -> Result<RocketConfig> {
        // Find the config file, starting from the `cwd` and working backwords.
        let file = RocketConfig::find()?;

        // Try to open the config file for reading.
        let mut handle = File::open(&file).map_err(|_| ConfigError::IOError)?;

        // Read the configure file to a string for parsing.
        let mut contents = String::new();
        handle.read_to_string(&mut contents).map_err(|_| ConfigError::IOError)?;

        // Parse the config and return the result.
        RocketConfig::parse(contents, &file)
    }

    /// Return the default configuration for all environments and marks the
    /// active environment (via the CONFIG_ENV variable) as active.
    pub fn active_default<P: AsRef<Path>>(filename: P) -> Result<RocketConfig> {
        let mut defaults = HashMap::new();
        defaults.insert(Development, Config::default(Development, &filename)?);
        defaults.insert(Staging, Config::default(Staging, &filename)?);
        defaults.insert(Production, Config::default(Production, &filename)?);

        let mut config = RocketConfig {
            active_env: Environment::active()?,
            config: defaults,
        };

        // Override any variables from the environment.
        config.override_from_env()?;
        Ok(config)
    }

    /// Iteratively search for `CONFIG_FILENAME` starting at the current working
    /// directory and working up through its parents. Returns the path to the
    /// file or an Error::NoKey if the file couldn't be found. If the current
    /// working directory can't be determined, return `BadCWD`.
    fn find() -> Result<PathBuf> {
        let cwd = env::current_dir().map_err(|_| ConfigError::BadCWD)?;
        let mut current = cwd.as_path();

        loop {
            let manifest = current.join(CONFIG_FILENAME);
            if fs::metadata(&manifest).is_ok() {
                return Ok(manifest)
            }

            match current.parent() {
                Some(p) => current = p,
                None => break,
            }
        }

        Err(ConfigError::NotFound)
    }

    fn get_mut(&mut self, env: Environment) -> &mut Config {
        match self.config.get_mut(&env) {
            Some(config) => config,
            None => panic!("set(): {} config is missing.", env),
        }
    }

    /// Set the configuration for the environment `env` to be the configuration
    /// derived from the TOML table `kvs`. The environment must already exist in
    /// `self`, otherwise this function panics. Any existing values are
    /// overriden by those in `kvs`.
    fn set_from_table(&mut self, env: Environment, kvs: &Table) -> Result<()> {
        for (key, value) in kvs {
            self.get_mut(env).set_raw(key, value)?;
        }

        Ok(())
    }

    /// Retrieves the `Config` for the environment `env`.
    pub fn get(&self, env: Environment) -> &Config {
        match self.config.get(&env) {
            Some(config) => config,
            None => panic!("get(): {} config is missing.", env),
        }
    }

    /// Retrieves the `Config` for the active environment.
    pub fn active(&self) -> &Config {
        self.get(self.active_env)
    }

    // Override all environments with values from env variables if present.
    fn override_from_env(&mut self) -> Result<()> {
        'outer: for (env_key, env_val) in env::vars() {
            if env_key.len() < ENV_VAR_PREFIX.len() {
                continue
            } else if !uncased_eq(&env_key[..ENV_VAR_PREFIX.len()], ENV_VAR_PREFIX) {
                continue
            }

            // Skip environment variables that are handled elsewhere.
            for prehandled_var in PREHANDLED_VARS.iter() {
                if uncased_eq(&env_key, &prehandled_var) {
                    continue 'outer
                }
            }

            // Parse the key and value and try to set the variable for all envs.
            let key = env_key[ENV_VAR_PREFIX.len()..].to_lowercase();
            let val = parse_simple_toml_value(&env_val);
            for env in &Environment::all() {
                match self.get_mut(*env).set_raw(&key, &val) {
                    Err(ConfigError::BadType(_, exp, _, _)) => {
                        return Err(ConfigError::BadEnvVal(env_key, env_val, exp))
                    }
                    Err(e) => return Err(e),
                    Ok(_) => { /* move along */ }
                }
            }
        }

        Ok(())
    }

    /// Parses the configuration from the Rocket.toml file. Also overrides any
    /// values there with values from the environment.
    fn parse<P: AsRef<Path>>(src: String, filename: P) -> Result<RocketConfig> {
        // Get a PathBuf version of the filename.
        let path = filename.as_ref().to_path_buf();

        // Parse the source as TOML, if possible.
        let mut parser = toml::Parser::new(&src);
        let toml = parser.parse().ok_or_else(|| {
            let source = src.clone();
            let errors = parser.errors.iter()
                .map(|error| ParsingError {
                    byte_range: (error.lo, error.hi),
                    start: parser.to_linecol(error.lo),
                    end: parser.to_linecol(error.hi),
                    desc: error.desc.clone(),
                });

            ConfigError::ParseError(source, path.clone(), errors.collect())
        })?;

        // Create a config with the defaults; set the env to the active one.
        let mut config = RocketConfig::active_default(filename)?;

        // Store all of the global overrides, if any, for later use.
        let mut global = None;

        // Parse the values from the TOML file.
        for (entry, value) in toml {
            // Each environment must be a table.
            let kv_pairs = match value.as_table() {
                Some(table) => table,
                None => return Err(ConfigError::BadType(
                    entry, "a table", value.type_str(), path.clone()
                ))
            };

            // Store the global table for later use and move on.
            if entry.as_str() == GLOBAL_ENV_NAME {
                global = Some(kv_pairs.clone());
                continue;
            }

            // This is not the global table. Parse the environment name from the
            // table entry name and then set all of the key/values.
            match entry.as_str().parse() {
                Ok(env) => config.set_from_table(env, kv_pairs)?,
                Err(_) => Err(ConfigError::BadEntry(entry.clone(), path.clone()))?
            }
        }

        // Override all of the environments with the global values.
        if let Some(ref global_kv_pairs) = global {
            for env in &Environment::all() {
                config.set_from_table(*env, global_kv_pairs)?;
            }
        }

        // Override any variables from the environment.
        config.override_from_env()?;

        Ok(config)
    }
}

/// Returns the active configuration and whether this call initialized the
/// configuration. The configuration can only be initialized once.
///
/// Initializes the global RocketConfig by reading the Rocket config file from
/// the current directory or any of its parents. Returns the active
/// configuration, which is determined by the config env variable. If there as a
/// problem parsing the configuration, the error is printed and the progam is
/// aborted. If there an I/O issue reading the config file, a warning is printed
/// and the default configuration is used. If there is no config file, the
/// default configuration is used.
///
/// # Panics
///
/// If there is a problem, prints a nice error message and bails.
pub(crate) fn init() -> (&'static Config, bool) {
    let mut this_init = false;
    unsafe {
        INIT.call_once(|| {
            private_init();
            this_init = true;
        });

        (CONFIG.as_ref().unwrap().active(), this_init)
    }
}

pub(crate) fn custom_init(config: Config) -> (&'static Config, bool) {
    let mut this_init = false;

    unsafe {
        INIT.call_once(|| {
            CONFIG = Some(RocketConfig::new(config));
            this_init = true;
        });

        (CONFIG.as_ref().unwrap().active(), this_init)
    }
}

unsafe fn private_init() {
    let bail = |e: ConfigError| -> ! {
        logger::init(LoggingLevel::Debug);
        e.pretty_print();
        process::exit(1)
    };

    use self::ConfigError::*;
    let config = RocketConfig::read().unwrap_or_else(|e| {
        match e {
            ParseError(..) | BadEntry(..) | BadEnv(..) | BadType(..)
                | BadFilePath(..) | BadEnvVal(..) => bail(e),
            IOError | BadCWD => warn!("Failed reading Rocket.toml. Using defaults."),
            NotFound => { /* try using the default below */ }
        }

        let default_path = match env::current_dir() {
            Ok(path) => path.join(&format!(".{}.{}", "default", CONFIG_FILENAME)),
            Err(_) => bail(ConfigError::BadCWD)
        };

        RocketConfig::active_default(&default_path).unwrap_or_else(|e| bail(e))
    });

    CONFIG = Some(config);
}

/// Retrieve the active configuration, if there is one.
///
/// This function is guaranteed to return `Some` once a Rocket application has
/// started. Before a Rocket application has started, or when there is no active
/// Rocket application (such as during testing), this function will return None.
pub fn active() -> Option<&'static Config> {
    unsafe { CONFIG.as_ref().map(|c| c.active()) }
}

#[cfg(test)]
mod test {
    use std::env;
    use std::sync::Mutex;

    use super::{RocketConfig, Config, ConfigError, ConfigBuilder};
    use super::{Environment, GLOBAL_ENV_NAME};
    use super::environment::CONFIG_ENV;
    use super::Environment::*;
    use super::Result;

    use ::logger::LoggingLevel;

    const TEST_CONFIG_FILENAME: &'static str = "/tmp/testing/Rocket.toml";

    // TODO: It's a shame we have to depend on lazy_static just for this.
    lazy_static! {
        static ref ENV_LOCK: Mutex<usize> = Mutex::new(0);
    }

    macro_rules! check_config {
        ($rconfig:expr, $econfig:expr) => (
            let expected = $econfig.finalize().unwrap();
            match $rconfig {
                Ok(config) => assert_eq!(config.active(), &expected),
                Err(e) => panic!("Config {} failed: {:?}", stringify!($rconfig), e)
            }
        );

        ($env:expr, $rconfig:expr, $econfig:expr) => (
            let expected = $econfig.finalize().unwrap();
            match $rconfig {
                Ok(ref config) => assert_eq!(config.get($env), &expected),
                Err(ref e) => panic!("Config {} failed: {:?}", stringify!($rconfig), e)
            }
        );
    }

    fn active_default() -> Result<RocketConfig>  {
        RocketConfig::active_default(TEST_CONFIG_FILENAME)
    }

    fn default_config(env: Environment) -> ConfigBuilder {
        ConfigBuilder::new(env)
    }

    #[test]
    fn test_defaults() {
        // Take the lock so changing the environment doesn't cause races.
        let _env_lock = ENV_LOCK.lock().unwrap();

        // First, without an environment. Should get development defaults.
        env::remove_var(CONFIG_ENV);
        check_config!(active_default(), default_config(Development));

        // Now with an explicit dev environment.
        for env in &["development", "dev"] {
            env::set_var(CONFIG_ENV, env);
            check_config!(active_default(), default_config(Development));
        }

        // Now staging.
        for env in &["stage", "staging"] {
            env::set_var(CONFIG_ENV, env);
            check_config!(active_default(), default_config(Staging));
        }

        // Finally, production.
        for env in &["prod", "production"] {
            env::set_var(CONFIG_ENV, env);
            check_config!(active_default(), default_config(Production));
        }
    }

    #[test]
    fn test_bad_environment_vars() {
        // Take the lock so changing the environment doesn't cause races.
        let _env_lock = ENV_LOCK.lock().unwrap();

        for env in &["", "p", "pr", "pro", "prodo", " prod", "dev ", "!dev!", "🚀 "] {
            env::set_var(CONFIG_ENV, env);
            let err = ConfigError::BadEnv(env.to_string());
            assert!(active_default().err().map_or(false, |e| e == err));
        }

        // Test that a bunch of invalid environment names give the right error.
        env::remove_var(CONFIG_ENV);
        for env in &["p", "pr", "pro", "prodo", "bad", "meow", "this", "that"] {
            let toml_table = format!("[{}]\n", env);
            let e_str = env.to_string();
            let err = ConfigError::BadEntry(e_str, TEST_CONFIG_FILENAME.into());
            assert!(RocketConfig::parse(toml_table, TEST_CONFIG_FILENAME)
                    .err().map_or(false, |e| e == err));
        }
    }

    #[test]
    fn test_good_full_config_files() {
        // Take the lock so changing the environment doesn't cause races.
        let _env_lock = ENV_LOCK.lock().unwrap();
        env::remove_var(CONFIG_ENV);

        let config_str = r#"
            address = "1.2.3.4"
            port = 7810
            workers = 21
            log = "critical"
            session_key = "8Xui8SN4mI+7egV/9dlfYYLGQJeEx4+DwmSQLwDVXJg="
            template_dir = "mine"
            json = true
            pi = 3.14
        "#;

        let mut expected = default_config(Development)
            .address("1.2.3.4")
            .port(7810)
            .workers(21)
            .log_level(LoggingLevel::Critical)
            .session_key("8Xui8SN4mI+7egV/9dlfYYLGQJeEx4+DwmSQLwDVXJg=")
            .extra("template_dir", "mine")
            .extra("json", true)
            .extra("pi", 3.14);

        expected.environment = Development;
        let dev_config = ["[dev]", config_str].join("\n");
        let parsed = RocketConfig::parse(dev_config, TEST_CONFIG_FILENAME);
        check_config!(Development, parsed, expected.clone());
        check_config!(Staging, parsed, default_config(Staging));
        check_config!(Production, parsed, default_config(Production));

        expected.environment = Staging;
        let stage_config = ["[stage]", config_str].join("\n");
        let parsed = RocketConfig::parse(stage_config, TEST_CONFIG_FILENAME);
        check_config!(Staging, parsed, expected.clone());
        check_config!(Development, parsed, default_config(Development));
        check_config!(Production, parsed, default_config(Production));

        expected.environment = Production;
        let prod_config = ["[prod]", config_str].join("\n");
        let parsed = RocketConfig::parse(prod_config, TEST_CONFIG_FILENAME);
        check_config!(Production, parsed, expected);
        check_config!(Development, parsed, default_config(Development));
        check_config!(Staging, parsed, default_config(Staging));
    }

    #[test]
    fn test_good_address_values() {
        // Take the lock so changing the environment doesn't cause races.
        let _env_lock = ENV_LOCK.lock().unwrap();
        env::set_var(CONFIG_ENV, "dev");

        check_config!(RocketConfig::parse(r#"
                          [development]
                          address = "localhost"
                      "#.to_string(), TEST_CONFIG_FILENAME), {
                          default_config(Development).address("localhost")
                      });

        check_config!(RocketConfig::parse(r#"
                          [development]
                          address = "127.0.0.1"
                      "#.to_string(), TEST_CONFIG_FILENAME), {
                          default_config(Development).address("127.0.0.1")
                      });

        check_config!(RocketConfig::parse(r#"
                          [development]
                          address = "::"
                      "#.to_string(), TEST_CONFIG_FILENAME), {
                          default_config(Development).address("::")
                      });

        check_config!(RocketConfig::parse(r#"
                          [dev]
                          address = "2001:db8::370:7334"
                      "#.to_string(), TEST_CONFIG_FILENAME), {
                          default_config(Development).address("2001:db8::370:7334")
                      });

        check_config!(RocketConfig::parse(r#"
                          [dev]
                          address = "0.0.0.0"
                      "#.to_string(), TEST_CONFIG_FILENAME), {
                          default_config(Development).address("0.0.0.0")
                      });
    }

    #[test]
    fn test_bad_address_values() {
        // Take the lock so changing the environment doesn't cause races.
        let _env_lock = ENV_LOCK.lock().unwrap();
        env::remove_var(CONFIG_ENV);

        assert!(RocketConfig::parse(r#"
            [development]
            address = 0000
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());

        assert!(RocketConfig::parse(r#"
            [development]
            address = true
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());

        assert!(RocketConfig::parse(r#"
            [development]
            address = "........"
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());

        assert!(RocketConfig::parse(r#"
            [staging]
            address = "1.2.3.4:100"
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());
    }

    #[test]
    fn test_good_port_values() {
        // Take the lock so changing the environment doesn't cause races.
        let _env_lock = ENV_LOCK.lock().unwrap();
        env::set_var(CONFIG_ENV, "stage");

        check_config!(RocketConfig::parse(r#"
                          [stage]
                          port = 100
                      "#.to_string(), TEST_CONFIG_FILENAME), {
                          default_config(Staging).port(100)
                      });

        check_config!(RocketConfig::parse(r#"
                          [stage]
                          port = 6000
                      "#.to_string(), TEST_CONFIG_FILENAME), {
                          default_config(Staging).port(6000)
                      });

        check_config!(RocketConfig::parse(r#"
                          [stage]
                          port = 65535
                      "#.to_string(), TEST_CONFIG_FILENAME), {
                          default_config(Staging).port(65535)
                      });
    }

    #[test]
    fn test_bad_port_values() {
        // Take the lock so changing the environment doesn't cause races.
        let _env_lock = ENV_LOCK.lock().unwrap();
        env::remove_var(CONFIG_ENV);

        assert!(RocketConfig::parse(r#"
            [development]
            port = true
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());

        assert!(RocketConfig::parse(r#"
            [production]
            port = "hello"
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());

        assert!(RocketConfig::parse(r#"
            [staging]
            port = -1
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());

        assert!(RocketConfig::parse(r#"
            [staging]
            port = 65536
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());

        assert!(RocketConfig::parse(r#"
            [staging]
            port = 105836
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());
    }

    #[test]
    fn test_good_workers_values() {
        // Take the lock so changing the environment doesn't cause races.
        let _env_lock = ENV_LOCK.lock().unwrap();
        env::set_var(CONFIG_ENV, "stage");

        check_config!(RocketConfig::parse(r#"
                          [stage]
                          workers = 1
                      "#.to_string(), TEST_CONFIG_FILENAME), {
                          default_config(Staging).workers(1)
                      });

        check_config!(RocketConfig::parse(r#"
                          [stage]
                          workers = 300
                      "#.to_string(), TEST_CONFIG_FILENAME), {
                          default_config(Staging).workers(300)
                      });

        check_config!(RocketConfig::parse(r#"
                          [stage]
                          workers = 65535
                      "#.to_string(), TEST_CONFIG_FILENAME), {
                          default_config(Staging).workers(65535)
                      });
    }

    #[test]
    fn test_bad_workers_values() {
        // Take the lock so changing the environment doesn't cause races.
        let _env_lock = ENV_LOCK.lock().unwrap();
        env::remove_var(CONFIG_ENV);

        assert!(RocketConfig::parse(r#"
            [development]
            workers = true
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());

        assert!(RocketConfig::parse(r#"
            [production]
            workers = "hello"
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());

        assert!(RocketConfig::parse(r#"
            [staging]
            workers = -1
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());

        assert!(RocketConfig::parse(r#"
            [staging]
            workers = 65536
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());

        assert!(RocketConfig::parse(r#"
            [staging]
            workers = 105836
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());
    }

    #[test]
    fn test_good_log_levels() {
        // Take the lock so changing the environment doesn't cause races.
        let _env_lock = ENV_LOCK.lock().unwrap();
        env::set_var(CONFIG_ENV, "stage");

        check_config!(RocketConfig::parse(r#"
                          [stage]
                          log = "normal"
                      "#.to_string(), TEST_CONFIG_FILENAME), {
                          default_config(Staging).log_level(LoggingLevel::Normal)
                      });


        check_config!(RocketConfig::parse(r#"
                          [stage]
                          log = "debug"
                      "#.to_string(), TEST_CONFIG_FILENAME), {
                          default_config(Staging).log_level(LoggingLevel::Debug)
                      });

        check_config!(RocketConfig::parse(r#"
                          [stage]
                          log = "critical"
                      "#.to_string(), TEST_CONFIG_FILENAME), {
                          default_config(Staging).log_level(LoggingLevel::Critical)
                      });
    }

    #[test]
    fn test_bad_log_level_values() {
        // Take the lock so changing the environment doesn't cause races.
        let _env_lock = ENV_LOCK.lock().unwrap();
        env::remove_var(CONFIG_ENV);

        assert!(RocketConfig::parse(r#"
            [dev]
            log = false
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());

        assert!(RocketConfig::parse(r#"
            [development]
            log = 0
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());

        assert!(RocketConfig::parse(r#"
            [prod]
            log = "no"
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());
    }

    #[test]
    fn test_good_session_key() {
        // Take the lock so changing the environment doesn't cause races.
        let _env_lock = ENV_LOCK.lock().unwrap();
        env::set_var(CONFIG_ENV, "stage");

        check_config!(RocketConfig::parse(r#"
                          [stage]
                          session_key = "TpUiXK2d/v5DFxJnWL12suJKPExKR8h9zd/o+E7SU+0="
                      "#.to_string(), TEST_CONFIG_FILENAME), {
                          default_config(Staging).session_key(
                              "TpUiXK2d/v5DFxJnWL12suJKPExKR8h9zd/o+E7SU+0="
                          )
                      });

        check_config!(RocketConfig::parse(r#"
                          [stage]
                          session_key = "jTyprDberFUiUFsJ3vcb1XKsYHWNBRvWAnXTlbTgGFU="
                      "#.to_string(), TEST_CONFIG_FILENAME), {
                          default_config(Staging).session_key(
                              "jTyprDberFUiUFsJ3vcb1XKsYHWNBRvWAnXTlbTgGFU="
                          )
                      });
    }

    #[test]
    fn test_bad_session_key() {
        // Take the lock so changing the environment doesn't cause races.
        let _env_lock = ENV_LOCK.lock().unwrap();
        env::remove_var(CONFIG_ENV);

        assert!(RocketConfig::parse(r#"
            [dev]
            session_key = true
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());

        assert!(RocketConfig::parse(r#"
            [dev]
            session_key = 1283724897238945234897
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());

        assert!(RocketConfig::parse(r#"
            [dev]
            session_key = "abcv"
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());
    }

    #[test]
    fn test_bad_toml() {
        // Take the lock so changing the environment doesn't cause races.
        let _env_lock = ENV_LOCK.lock().unwrap();
        env::remove_var(CONFIG_ENV);

        assert!(RocketConfig::parse(r#"
            [dev
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());

        assert!(RocketConfig::parse(r#"
            [dev]
            1.2.3 = 2
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());

        assert!(RocketConfig::parse(r#"
            [dev]
            session_key = "abcv" = other
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());
    }

    #[test]
    fn test_global_overrides() {
        // Take the lock so changing the environment doesn't cause races.
        let _env_lock = ENV_LOCK.lock().unwrap();

        // Test first that we can override each environment.
        for env in &Environment::all() {
            env::set_var(CONFIG_ENV, env.to_string());

            check_config!(RocketConfig::parse(format!(r#"
                              [{}]
                              address = "::1"
                          "#, GLOBAL_ENV_NAME), TEST_CONFIG_FILENAME), {
                              default_config(*env).address("::1")
                          });

            check_config!(RocketConfig::parse(format!(r#"
                              [{}]
                              database = "mysql"
                          "#, GLOBAL_ENV_NAME), TEST_CONFIG_FILENAME), {
                              default_config(*env).extra("database", "mysql")
                          });

            check_config!(RocketConfig::parse(format!(r#"
                              [{}]
                              port = 3980
                          "#, GLOBAL_ENV_NAME), TEST_CONFIG_FILENAME), {
                              default_config(*env).port(3980)
                          });
        }
    }

    #[test]
    fn test_env_override() {
        // Take the lock so changing the environment doesn't cause races.
        let _env_lock = ENV_LOCK.lock().unwrap();

        let pairs = [
            ("log", "critical"), ("LOG", "debug"), ("PORT", "8110"),
            ("address", "1.2.3.4"), ("EXTRA_EXTRA", "true"), ("workers", "3")
        ];

        let check_value = |key: &str, val: &str, config: &Config| {
            match key {
                "log" => assert_eq!(config.log_level, val.parse().unwrap()),
                "port" => assert_eq!(config.port, val.parse().unwrap()),
                "address" => assert_eq!(config.address, val),
                "extra_extra" => assert_eq!(config.get_bool(key).unwrap(), true),
                "workers" => assert_eq!(config.workers, val.parse().unwrap()),
                _ => panic!("Unexpected key: {}", key)
            }
        };

        // Check that setting the environment variable actually changes the
        // config for the default active and nonactive environments.
        for &(key, val) in &pairs {
            env::set_var(format!("ROCKET_{}", key), val);

            let rconfig = active_default().unwrap();
            // Check that it overrides the active config.
            for env in &Environment::all() {
                env::set_var(CONFIG_ENV, env.to_string());
                let rconfig = active_default().unwrap();
                check_value(&*key.to_lowercase(), val, rconfig.active());
            }

            // And non-active configs.
            for env in &Environment::all() {
                check_value(&*key.to_lowercase(), val, rconfig.get(*env));
            }
        }

        // Clear the variables so they don't override for the next test.
        for &(key, _) in &pairs {
            env::remove_var(format!("ROCKET_{}", key))
        }

        // Now we build a config file to test that the environment variables
        // override configurations from files as well.
        let toml = r#"
            [dev]
            address = "1.2.3.4"

            [stage]
            address = "2.3.4.5"

            [prod]
            address = "10.1.1.1"

            [global]
            address = "1.2.3.4"
            port = 7810
            workers = 21
            log = "normal"
        "#.to_string();

        // Check that setting the environment variable actually changes the
        // config for the default active environments.
        for &(key, val) in &pairs {
            env::set_var(format!("ROCKET_{}", key), val);

            let r = RocketConfig::parse(toml.clone(), TEST_CONFIG_FILENAME).unwrap();
            check_value(&*key.to_lowercase(), val, r.active());

            // And non-active configs.
            for env in &Environment::all() {
                check_value(&*key.to_lowercase(), val, r.get(*env));
            }
        }

        // Clear the variables so they don't override for the next test.
        for &(key, _) in &pairs {
            env::remove_var(format!("ROCKET_{}", key))
        }
    }
}
