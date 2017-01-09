//! Application configuration and configuration parameter retrieval.
//!
//! This module implements configuration handling for Rocket. It implements
//! the parsing and interpretation of the `Rocket.toml` config file. It also
//! allows libraries to access values that have been configured by the user.
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
//!   * **log**: _[string]_ how much information to log; one of `"normal"`,
//!     `"debug"`, or `"critical"`
//!   * **session_key**: _[string]_ a 192-bit base64 encoded string (32
//!     characters) to use as the session key
//!     * example: `"VheMwXIBygSmOlZAhuWl2B+zgvTN3WW5"`
//!
//! ### Rocket.toml
//!
//! The `Rocket.toml` file is used to specify the configuration parameters for
//! each environment. The file is optional. If it is not present, the default
//! configuration parameters are used.
//!
//! The file must be a series of tables, at most one for each environment and a
//! "global" table, where each table contains key-value pairs corresponding to
//! configuration parameters for that environment. If a configuration parameter
//! is missing, the default value is used. The following is a complete
//! `Rocket.toml` file, where every standard configuration parameter is
//! specified with the default value:
//!
//! ```toml
//! [development]
//! address = "localhost"
//! port = 8000
//! log = "normal"
//!
//! [staging]
//! address = "0.0.0.0"
//! port = 80
//! log = "normal"
//! # don't use this key! generate your own and keep it private!
//! session_key = "VheMwXIBygSmOlZAhuWl2B+zgvTN3WW5"
//!
//! [production]
//! address = "0.0.0.0"
//! port = 80
//! log = "critical"
//! # don't use this key! generate your own and keep it private!
//! session_key = "adL5fFIPmZBrlyHk2YT4NLV3YCk2gFXz"
//! ```
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
//! use rocket::config;
//!
//! const DEFAULT_TEMPLATE_DIR: &'static str = "templates";
//!
//! let template_dir = config::active().map(|config| {
//!     let dir = config.get_str("template_dir")
//!         .map_err(|e| if !e.is_not_found() { e.pretty_print(); })
//!         .unwrap_or(DEFAULT_TEMPLATE_DIR);
//!
//!     config.root().join(dir).to_string_lossy().into_owned()
//! }).unwrap_or(DEFAULT_TEMPLATE_DIR.to_string());
//! ```
//!
//! Libraries should always use a default if a parameter is not defined.

mod error;
mod environment;
mod config;

use std::sync::{Once, ONCE_INIT};
use std::fs::{self, File};
use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::process;
use std::env;

pub use self::error::{ConfigError, ParsingError};
pub use self::environment::Environment;
pub use self::config::Config;
use self::Environment::*;

use toml;
pub use toml::{Array, Table, Value};

use logger::{self, LoggingLevel};

static INIT: Once = ONCE_INIT;
static mut CONFIG: Option<RocketConfig> = None;

const CONFIG_FILENAME: &'static str = "Rocket.toml";

const GLOBAL_ENV_NAME: &'static str = "global";

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
    /// Iteratively search for `file` in `pwd` and its parents, returning the path
    /// to the file or an Error::NoKey if the file couldn't be found.
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

    fn set(&mut self, env: Environment, kvs: &Table)
            -> Result<()> {
        let config = match self.config.get_mut(&env) {
            Some(config) => config,
            None => panic!("set(): {} config is missing.", env),
        };

        for (key, value) in kvs {
            config.set(key, value)?;
        }

        Ok(())
    }

    pub fn get(&self, env: Environment) -> &Config {
        match self.config.get(&env) {
            Some(config) => config,
            None => panic!("get(): {} config is missing.", env),
        }
    }

    pub fn active(&self) -> &Config {
        self.get(self.active_env)
    }

    fn parse(src: String, filename: &str) -> Result<RocketConfig> {
        // Parse the source as TOML, if possible.
        let mut parser = toml::Parser::new(&src);
        let toml = parser.parse().ok_or_else(|| ConfigError::ParseError(
            src.clone(), filename.into(),
            parser.errors.iter().map(|error| ParsingError {
                byte_range: (error.lo, error.hi),
                start: parser.to_linecol(error.lo),
                end: parser.to_linecol(error.hi),
                desc: error.desc.clone(),
            }).collect()
        ))?;

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
                    entry, "a table", value.type_str(), filename.into()
                ))
            };

            if entry.as_str() == GLOBAL_ENV_NAME {
                global = Some(kv_pairs.clone());
            } else {
                // Parse the environment from the table entry name.
                let env = entry.as_str().parse().map_err(|_| {
                    ConfigError::BadEntry(entry.clone(), filename.into())
                })?;

                // Set the environment configuration from the kv pairs.
                config.set(env, kv_pairs)?;
            }
        }

        // Override all of the environments with the global values.
        if let Some(ref global_kv_pairs) = global {
            for env in &Environment::all() {
                config.set(*env, global_kv_pairs)?;
            }
        }

        Ok(config)
    }

    pub fn read() -> Result<RocketConfig> {
        // Find the config file, starting from the `cwd` and working backwords.
        let file = RocketConfig::find()?;

        // Try to open the config file for reading.
        let mut handle = File::open(&file).map_err(|_| ConfigError::IOError)?;

        // Read the configure file to a string for parsing.
        let mut contents = String::new();
        handle.read_to_string(&mut contents).map_err(|_| ConfigError::IOError)?;

        // Parse the contents from the file.
        RocketConfig::parse(contents, &file.to_string_lossy())
    }

    pub fn active_default(filename: &str) -> Result<RocketConfig> {
        let mut defaults = HashMap::new();
        defaults.insert(Development, Config::default_for(Development, filename)?);
        defaults.insert(Staging, Config::default_for(Staging, filename)?);
        defaults.insert(Production, Config::default_for(Production, filename)?);

        Ok(RocketConfig {
            active_env: Environment::active()?,
            config: defaults,
        })
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
#[doc(hidden)]
pub fn init() -> (&'static Config, bool) {
    let mut this_init = false;
    unsafe {
        INIT.call_once(|| {
            private_init();
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
                | BadFilePath(..) => bail(e),
            IOError | BadCWD => warn!("Failed reading Rocket.toml. Using defaults."),
            NotFound => { /* try using the default below */ }
        }

        let default_path = match env::current_dir() {
            Ok(path) => path.join(&format!(".{}.{}", "default", CONFIG_FILENAME)),
            Err(_) => bail(ConfigError::BadCWD)
        };

        let filename = default_path.to_string_lossy();
        RocketConfig::active_default(&filename).unwrap_or_else(|e| bail(e))
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

    use super::{RocketConfig, ConfigError, GLOBAL_ENV_NAME};
    use super::environment::{Environment, CONFIG_ENV};
    use super::Environment::*;
    use super::config::Config;
    use super::Result;

    use ::toml::Value;
    use ::logger::LoggingLevel;

    const TEST_CONFIG_FILENAME: &'static str = "/tmp/testing/Rocket.toml";

    lazy_static! {
        static ref ENV_LOCK: Mutex<usize> = Mutex::new(0);
    }

    macro_rules! check_config {
        ($rconfig:expr, $econfig:expr) => (
            match $rconfig {
                Ok(config) => assert_eq!(config.active(), &$econfig),
                Err(e) => panic!("Config {} failed: {:?}", stringify!($rconfig), e)
            }
        );

        ($env:expr, $rconfig:expr, $econfig:expr) => (
            match $rconfig {
                Ok(ref config) => assert_eq!(config.get($env), &$econfig),
                Err(ref e) => panic!("Config {} failed: {:?}", stringify!($rconfig), e)
            }
        );
    }

    fn active_default() -> Result<RocketConfig>  {
        RocketConfig::active_default(TEST_CONFIG_FILENAME)
    }

    fn default_config(env: Environment) -> Config {
        Config::default_for(env, TEST_CONFIG_FILENAME).expect("config")
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
            log = "critical"
            session_key = "01234567890123456789012345678901"
            template_dir = "mine"
            json = true
            pi = 3.14
        "#;

        let mut expected = default_config(Development)
                .address("1.2.3.4".to_string())
                .port(7810)
                .log_level(LoggingLevel::Critical)
                .session_key("01234567890123456789012345678901".into());

        expected.set("template_dir", &Value::String("mine".into())).unwrap();
        expected.set("json", &Value::Boolean(true)).unwrap();
        expected.set("pi", &Value::Float(3.14)).unwrap();

        expected.env = Development;
        let dev_config = ["[dev]", config_str].join("\n");
        let parsed = RocketConfig::parse(dev_config, TEST_CONFIG_FILENAME);
        check_config!(Development, parsed, expected);
        check_config!(Staging, parsed, default_config(Staging));
        check_config!(Production, parsed, default_config(Production));

        expected.env = Staging;
        let stage_config = ["[stage]", config_str].join("\n");
        let parsed = RocketConfig::parse(stage_config, TEST_CONFIG_FILENAME);
        check_config!(Staging, parsed, expected);
        check_config!(Development, parsed, default_config(Development));
        check_config!(Production, parsed, default_config(Production));

        expected.env = Production;
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
                          default_config(Development).address("localhost".into())
                      });

        check_config!(RocketConfig::parse(r#"
                          [dev]
                          address = "127.0.0.1"
                      "#.to_string(), TEST_CONFIG_FILENAME), {
                          default_config(Development).address("127.0.0.1".into())
                      });

        check_config!(RocketConfig::parse(r#"
                          [dev]
                          address = "0.0.0.0"
                      "#.to_string(), TEST_CONFIG_FILENAME), {
                          default_config(Development).address("0.0.0.0".into())
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
                          session_key = "VheMwXIBygSmOlZAhuWl2B+zgvTN3WW5"
                      "#.to_string(), TEST_CONFIG_FILENAME), {
                          default_config(Staging).session_key(
                              "VheMwXIBygSmOlZAhuWl2B+zgvTN3WW5".into()
                          )
                      });

        check_config!(RocketConfig::parse(r#"
                          [stage]
                          session_key = "adL5fFIPmZBrlyHk2YT4NLV3YCk2gFXz"
                      "#.to_string(), TEST_CONFIG_FILENAME), {
                          default_config(Staging).session_key(
                              "adL5fFIPmZBrlyHk2YT4NLV3YCk2gFXz".into()
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
                              address = "7.6.5.4"
                          "#, GLOBAL_ENV_NAME), TEST_CONFIG_FILENAME), {
                              default_config(*env).address(
                                  "7.6.5.4".into()
                              )
                          });

            check_config!(RocketConfig::parse(format!(r#"
                              [{}]
                              database = "mysql"
                          "#, GLOBAL_ENV_NAME), TEST_CONFIG_FILENAME), {
                              default_config(*env).extra("database",
                                  &Value::String("mysql".into())
                              )
                          });

            check_config!(RocketConfig::parse(format!(r#"
                              [{}]
                              port = 3980
                          "#, GLOBAL_ENV_NAME), TEST_CONFIG_FILENAME), {
                              default_config(*env).port(
                                  3980
                              )
                          });
        }
    }
}
