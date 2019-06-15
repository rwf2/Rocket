//! Application configuration and configuration parameter retrieval.
//!
//! This module implements configuration handling for Rocket. It implements the
//! parsing and interpretation of the `Rocket.toml` config file and
//! `ROCKET_{PARAM}` environment variables. It also allows libraries to access
//! user-configured values.
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
//! | name       | type           | description                                                 | examples                   |
//! |------------|----------------|-------------------------------------------------------------|----------------------------|
//! | address    | string         | ip address or host to listen on                             | `"localhost"`, `"1.2.3.4"` |
//! | port       | integer        | port number to listen on                                    | `8000`, `80`               |
//! | keep_alive | integer        | keep-alive timeout in seconds                               | `0` (disable), `10`        |
//! | workers    | integer        | number of concurrent thread workers                         | `36`, `512`                |
//! | log        | string         | max log level: `"off"`, `"normal"`, `"debug"`, `"critical"` | `"off"`, `"normal"`        |
//! | secret_key | 256-bit base64 | secret key for private cookies                              | `"8Xui8SI..."` (44 chars)  |
//! | tls        | table          | tls config table with two keys (`certs`, `key`)             | _see below_                |
//! | tls.certs  | string         | path to certificate chain in PEM format                     | `"private/cert.pem"`       |
//! | tls.key    | string         | path to private key for `tls.certs` in PEM format           | `"private/key.pem"`        |
//! | limits     | table          | map from data type (string) to data limit (integer: bytes)  | `{ forms = 65536 }`        |
//!
//! ### Rocket.toml
//!
//! `Rocket.toml` is a Rocket application's configuration file. It can
//! optionally be used to specify the configuration parameters for each
//! environment. If it is not present, the default configuration parameters or
//! environment supplied parameters are used.
//!
//! The file must be a series of TOML tables, at most one for each environment,
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
//! workers = [number_of_cpus * 2]
//! keep_alive = 5
//! log = "normal"
//! secret_key = [randomly generated at launch]
//! limits = { forms = 32768 }
//!
//! [staging]
//! address = "0.0.0.0"
//! port = 8000
//! workers = [number_of_cpus * 2]
//! keep_alive = 5
//! log = "normal"
//! secret_key = [randomly generated at launch]
//! limits = { forms = 32768 }
//!
//! [production]
//! address = "0.0.0.0"
//! port = 8000
//! workers = [number_of_cpus * 2]
//! keep_alive = 5
//! log = "critical"
//! secret_key = [randomly generated at launch]
//! limits = { forms = 32768 }
//! ```
//!
//! The `workers` and `secret_key` default parameters are computed by Rocket
//! automatically; the values above are not valid TOML syntax. When manually
//! specifying the number of workers, the value should be an integer: `workers =
//! 10`. When manually specifying the secret key, the value should a 256-bit
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
//! ### TLS Configuration
//!
//! TLS can be enabled by specifying the `tls.key` and `tls.certs` parameters.
//! Rocket must be compiled with the `tls` feature enabled for the parameters to
//! take effect. The recommended way to specify the parameters is via the
//! `global` environment:
//!
//! ```toml
//! [global.tls]
//! certs = "/path/to/certs.pem"
//! key = "/path/to/key.pem"
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
//! Variable values are parsed as if they were TOML syntax. As illustration,
//! consider the following examples:
//!
//! ```sh
//! ROCKET_INTEGER=1
//! ROCKET_FLOAT=3.14
//! ROCKET_STRING=Hello
//! ROCKET_STRING="Hello"
//! ROCKET_BOOL=true
//! ROCKET_ARRAY=[1,"b",3.14]
//! ROCKET_DICT={key="abc",val=123}
//! ```
//!
//! ## Retrieving Configuration Parameters
//!
//! Configuration parameters for the currently active configuration environment
//! can be retrieved via the [`Rocket::config()`] `Rocket` and `get_` methods on
//! [`Config`] structure.
//!
//! The retrivial of configuration parameters usually occurs at launch time via
//! a [launch fairing](crate::fairing::Fairing). If information about the
//! configuraiton is needed later in the program, an attach fairing can be used
//! to store the information as managed state. As an example of the latter,
//! consider the following short program which reads the `token` configuration
//! parameter and stores the value or a default in a `Token` managed state
//! value:
//!
//! ```rust
//! use rocket::fairing::AdHoc;
//!
//! struct Token(i64);
//!
//! fn main() {
//!     rocket::ignite()
//!         .attach(AdHoc::on_attach("Token Config", |rocket| {
//!             println!("Adding token managed state from config...");
//!             let token_val = rocket.config().get_int("token").unwrap_or(-1);
//!             Ok(rocket.manage(Token(token_val)))
//!         }))
//! # ;
//! }
//! ```

mod error;
mod environment;
mod config;
mod builder;
mod toml_ext;
mod custom_values;

use std::fs::{self, File};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process;
use std::env;

use toml;

pub use self::custom_values::Limits;
pub use toml::value::{Array, Table, Value, Datetime};
pub use self::error::ConfigError;
pub use self::environment::Environment;
pub use self::config::Config;
pub use self::builder::ConfigBuilder;
pub use crate::logger::LoggingLevel;
crate use self::toml_ext::LoggedValue;

use crate::logger;
use self::Environment::*;
use self::environment::CONFIG_ENV;
use crate::logger::COLORS_ENV;
use self::toml_ext::parse_simple_toml_value;
use crate::http::uncased::uncased_eq;

const CONFIG_FILENAME: &str = "Rocket.toml";
const GLOBAL_ENV_NAME: &str = "global";
const ENV_VAR_PREFIX: &str = "ROCKET_";
const PREHANDLED_VARS: [&str; 3] = ["ROCKET_CODEGEN_DEBUG", CONFIG_ENV, COLORS_ENV];

/// Wraps `std::result` with the error type of [`ConfigError`].
pub type Result<T> = std::result::Result<T, ConfigError>;

#[doc(hidden)]
#[derive(Debug, PartialEq)]
pub struct RocketConfig {
    pub active_env: Environment,
    config: HashMap<Environment, Config>,
}

impl RocketConfig {
    /// Read the configuration from the `Rocket.toml` file. The file is search
    /// for recursively up the tree, starting from the CWD.
    pub fn read() -> Result<RocketConfig> {
        // Find the config file, starting from the `cwd` and working backwards.
        let file = RocketConfig::find()?;

        // Try to open the config file for reading.
        let mut handle = File::open(&file).map_err(|_| ConfigError::IoError)?;

        // Read the configure file to a string for parsing.
        let mut contents = String::new();
        handle.read_to_string(&mut contents).map_err(|_| ConfigError::IoError)?;

        // Parse the config and return the result.
        RocketConfig::parse(contents, &file)
    }

    /// Return the default configuration for all environments and marks the
    /// active environment (via the CONFIG_ENV variable) as active.
    pub fn active_default_from(filename: Option<&Path>) -> Result<RocketConfig> {
        let mut defaults = HashMap::new();
        if let Some(path) = filename {
            defaults.insert(Development, Config::default_from(Development, &path)?);
            defaults.insert(Staging, Config::default_from(Staging, &path)?);
            defaults.insert(Production, Config::default_from(Production, &path)?);
        } else {
            defaults.insert(Development, Config::default(Development));
            defaults.insert(Staging, Config::default(Staging));
            defaults.insert(Production, Config::default(Production));
        }

        let mut config = RocketConfig {
            active_env: Environment::active()?,
            config: defaults,
        };

        // Override any variables from the environment.
        config.override_from_env()?;
        Ok(config)
    }

    /// Return the default configuration for all environments and marks the
    /// active environment (via the CONFIG_ENV variable) as active.
    pub fn active_default() -> Result<RocketConfig> {
        RocketConfig::active_default_from(None)
    }

    /// Iteratively search for `CONFIG_FILENAME` starting at the current working
    /// directory and working up through its parents. Returns the path to the
    /// file or an Error::NoKey if the file couldn't be found. If the current
    /// working directory can't be determined, return `BadCWD`.
    fn find() -> Result<PathBuf> {
        let cwd = env::current_dir().map_err(|_| ConfigError::NotFound)?;
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

    #[inline]
    fn get_mut(&mut self, env: Environment) -> &mut Config {
        match self.config.get_mut(&env) {
            Some(config) => config,
            None => panic!("set(): {} config is missing.", env),
        }
    }

    /// Set the configuration for the environment `env` to be the configuration
    /// derived from the TOML table `kvs`. The environment must already exist in
    /// `self`, otherwise this function panics. Any existing values are
    /// overridden by those in `kvs`.
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
    #[inline]
    pub fn active(&self) -> &Config {
        self.get(self.active_env)
    }

    // Override all environments with values from env variables if present.
    fn override_from_env(&mut self) -> Result<()> {
        for (key, val) in env::vars() {
            if key.len() < ENV_VAR_PREFIX.len() {
                continue
            } else if !uncased_eq(&key[..ENV_VAR_PREFIX.len()], ENV_VAR_PREFIX) {
                continue
            }

            // Skip environment variables that are handled elsewhere.
            if PREHANDLED_VARS.iter().any(|var| uncased_eq(&key, var)) {
                continue
            }

            // Parse the key and value and try to set the variable for all envs.
            let key = key[ENV_VAR_PREFIX.len()..].to_lowercase();
            let toml_val = match parse_simple_toml_value(&val) {
                Ok(val) => val,
                Err(e) => return Err(ConfigError::BadEnvVal(key, val, e))
            };

            for env in &Environment::ALL {
                match self.get_mut(*env).set_raw(&key, &toml_val) {
                    Err(ConfigError::BadType(_, exp, actual, _)) => {
                        let e = format!("expected {}, but found {}", exp, actual);
                        return Err(ConfigError::BadEnvVal(key, val, e))
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
        use self::ConfigError::ParseError;

        // Parse the source as TOML, if possible.
        let path = filename.as_ref().to_path_buf();
        let table = match src.parse::<toml::Value>() {
            Ok(toml::Value::Table(table)) => table,
            Ok(value) => {
                let err = format!("expected a table, found {}", value.type_str());
                return Err(ConfigError::ParseError(src, path, err, Some((1, 1))));
            }
            Err(e) => return Err(ParseError(src, path, e.to_string(), e.line_col()))
        };

        // Create a config with the defaults; set the env to the active one.
        let mut config = RocketConfig::active_default_from(Some(filename.as_ref()))?;

        // Store all of the global overrides, if any, for later use.
        let mut global = None;

        // Parse the values from the TOML file.
        for (entry, value) in table {
            // Each environment must be a table.
            let kv_pairs = match value.as_table() {
                Some(table) => table,
                None => return Err(ConfigError::BadType(
                    entry, "a table", value.type_str(), Some(path.clone())
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
            for env in &Environment::ALL {
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
/// problem parsing the configuration, the error is printed and the program is
/// aborted. If there an I/O issue reading the config file, a warning is printed
/// and the default configuration is used. If there is no config file, the
/// default configuration is used.
///
/// # Panics
///
/// If there is a problem, prints a nice error message and bails.
crate fn init() -> Config {
    let bail = |e: ConfigError| -> ! {
        logger::init(LoggingLevel::Debug);
        e.pretty_print();
        process::exit(1)
    };

    use self::ConfigError::*;
    let config = RocketConfig::read().unwrap_or_else(|e| {
        match e {
            | ParseError(..) | BadEntry(..) | BadEnv(..) | BadType(..) | Io(..)
            | BadFilePath(..) | BadEnvVal(..) | UnknownKey(..)
            | Missing(..) => bail(e),
            IoError => warn!("Failed reading Rocket.toml. Using defaults."),
            NotFound => { /* try using the default below */ }
        }

        RocketConfig::active_default().unwrap_or_else(|e| bail(e))
    });

    // FIXME: Should probably store all of the config.
    config.active().clone()
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

    use crate::logger::LoggingLevel;

    const TEST_CONFIG_FILENAME: &'static str = "/tmp/testing/Rocket.toml";

    // TODO: It's a shame we have to depend on lazy_static just for this.
    lazy_static::lazy_static! {
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
        RocketConfig::active_default()
    }

    fn default_config(env: Environment) -> ConfigBuilder {
        ConfigBuilder::new(env)
    }

    #[test]
    fn test_defaults() {
        // Take the lock so changing the environment doesn't cause races.
        let _env_lock = ENV_LOCK.lock().unwrap();

        // First, without an environment. Should get development defaults on
        // debug builds and productions defaults on non-debug builds.
        env::remove_var(CONFIG_ENV);
        #[cfg(debug_assertions)] check_config!(active_default(), default_config(Development));
        #[cfg(not(debug_assertions))] check_config!(active_default(), default_config(Production));

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
            keep_alive = 0
            secret_key = "8Xui8SN4mI+7egV/9dlfYYLGQJeEx4+DwmSQLwDVXJg="
            template_dir = "mine"
            json = true
            pi = 3.14
        "#;

        let mut expected = default_config(Development)
            .address("1.2.3.4")
            .port(7810)
            .workers(21)
            .log_level(LoggingLevel::Critical)
            .keep_alive(0)
            .secret_key("8Xui8SN4mI+7egV/9dlfYYLGQJeEx4+DwmSQLwDVXJg=")
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

    // Only do this test when the tls feature is disabled since the file paths
    // we're supplying don't actually exist.
    #[test]
    fn test_good_tls_values() {
        // Take the lock so changing the environment doesn't cause races.
        let _env_lock = ENV_LOCK.lock().unwrap();
        env::set_var(CONFIG_ENV, "dev");

        assert!(RocketConfig::parse(r#"
            [staging]
            tls = { certs = "some/path.pem", key = "some/key.pem" }
        "#.to_string(), TEST_CONFIG_FILENAME).is_ok());

        assert!(RocketConfig::parse(r#"
            [staging.tls]
            certs = "some/path.pem"
            key = "some/key.pem"
        "#.to_string(), TEST_CONFIG_FILENAME).is_ok());

        assert!(RocketConfig::parse(r#"
            [global.tls]
            certs = "some/path.pem"
            key = "some/key.pem"
        "#.to_string(), TEST_CONFIG_FILENAME).is_ok());

        assert!(RocketConfig::parse(r#"
            [global]
            tls = { certs = "some/path.pem", key = "some/key.pem" }
        "#.to_string(), TEST_CONFIG_FILENAME).is_ok());
    }

    #[test]
    fn test_bad_tls_config() {
        // Take the lock so changing the environment doesn't cause races.
        let _env_lock = ENV_LOCK.lock().unwrap();
        env::remove_var(CONFIG_ENV);

        assert!(RocketConfig::parse(r#"
            [development]
            tls = "hello"
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());

        assert!(RocketConfig::parse(r#"
            [development]
            tls = { certs = "some/path.pem" }
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());

        assert!(RocketConfig::parse(r#"
            [development]
            tls = { certs = "some/path.pem", key = "some/key.pem", extra = "bah" }
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());

        assert!(RocketConfig::parse(r#"
            [staging]
            tls = { cert = "some/path.pem", key = "some/key.pem" }
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
    fn test_good_keep_alives() {
        // Take the lock so changing the environment doesn't cause races.
        let _env_lock = ENV_LOCK.lock().unwrap();
        env::set_var(CONFIG_ENV, "stage");

        check_config!(RocketConfig::parse(r#"
                          [stage]
                          keep_alive = 10
                      "#.to_string(), TEST_CONFIG_FILENAME), {
                          default_config(Staging).keep_alive(10)
                      });

        check_config!(RocketConfig::parse(r#"
                          [stage]
                          keep_alive = 0
                      "#.to_string(), TEST_CONFIG_FILENAME), {
                          default_config(Staging).keep_alive(0)
                      });

        check_config!(RocketConfig::parse(r#"
                          [stage]
                          keep_alive = 348
                      "#.to_string(), TEST_CONFIG_FILENAME), {
                          default_config(Staging).keep_alive(348)
                      });

        check_config!(RocketConfig::parse(r#"
                          [stage]
                          keep_alive = 0
                      "#.to_string(), TEST_CONFIG_FILENAME), {
                          default_config(Staging).keep_alive(0)
                      });
    }

    #[test]
    fn test_bad_keep_alives() {
        // Take the lock so changing the environment doesn't cause races.
        let _env_lock = ENV_LOCK.lock().unwrap();
        env::remove_var(CONFIG_ENV);

        assert!(RocketConfig::parse(r#"
            [dev]
            keep_alive = true
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());

        assert!(RocketConfig::parse(r#"
            [dev]
            keep_alive = -10
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());

        assert!(RocketConfig::parse(r#"
            [dev]
            keep_alive = "Some(10)"
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());

        assert!(RocketConfig::parse(r#"
            [dev]
            keep_alive = 4294967296
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

        check_config!(RocketConfig::parse(r#"
                          [stage]
                          log = "off"
                      "#.to_string(), TEST_CONFIG_FILENAME), {
                          default_config(Staging).log_level(LoggingLevel::Off)
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
    fn test_good_secret_key() {
        // Take the lock so changing the environment doesn't cause races.
        let _env_lock = ENV_LOCK.lock().unwrap();
        env::set_var(CONFIG_ENV, "stage");

        check_config!(RocketConfig::parse(r#"
                          [stage]
                          secret_key = "TpUiXK2d/v5DFxJnWL12suJKPExKR8h9zd/o+E7SU+0="
                      "#.to_string(), TEST_CONFIG_FILENAME), {
                          default_config(Staging).secret_key(
                              "TpUiXK2d/v5DFxJnWL12suJKPExKR8h9zd/o+E7SU+0="
                          )
                      });

        check_config!(RocketConfig::parse(r#"
                          [stage]
                          secret_key = "jTyprDberFUiUFsJ3vcb1XKsYHWNBRvWAnXTlbTgGFU="
                      "#.to_string(), TEST_CONFIG_FILENAME), {
                          default_config(Staging).secret_key(
                              "jTyprDberFUiUFsJ3vcb1XKsYHWNBRvWAnXTlbTgGFU="
                          )
                      });
    }

    #[test]
    fn test_bad_secret_key() {
        // Take the lock so changing the environment doesn't cause races.
        let _env_lock = ENV_LOCK.lock().unwrap();
        env::remove_var(CONFIG_ENV);

        assert!(RocketConfig::parse(r#"
            [dev]
            secret_key = true
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());

        assert!(RocketConfig::parse(r#"
            [dev]
            secret_key = 1283724897238945234897
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());

        assert!(RocketConfig::parse(r#"
            [dev]
            secret_key = "abcv"
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
            1. = 2
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());

        assert!(RocketConfig::parse(r#"
            [dev]
            secret_key = "abcv" = other
        "#.to_string(), TEST_CONFIG_FILENAME).is_err());
    }

    #[test]
    fn test_global_overrides() {
        // Take the lock so changing the environment doesn't cause races.
        let _env_lock = ENV_LOCK.lock().unwrap();

        // Test first that we can override each environment.
        for env in &Environment::ALL {
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
            for env in &Environment::ALL {
                env::set_var(CONFIG_ENV, env.to_string());
                let rconfig = active_default().unwrap();
                check_value(&*key.to_lowercase(), val, rconfig.active());
            }

            // And non-active configs.
            for env in &Environment::ALL {
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
            for env in &Environment::ALL {
                check_value(&*key.to_lowercase(), val, r.get(*env));
            }
        }

        // Clear the variables so they don't override for the next test.
        for &(key, _) in &pairs {
            env::remove_var(format!("ROCKET_{}", key))
        }
    }
}
