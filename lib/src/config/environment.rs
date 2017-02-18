use super::ConfigError;

use std::fmt;
use std::str::FromStr;
use std::env;

use self::Environment::*;

pub const CONFIG_ENV: &'static str = "ROCKET_ENV";

/// An enum corresponding to the valid configuration environments.
#[derive(Hash, PartialEq, Eq, Debug, Clone, Copy)]
pub enum Environment {
    /// The development environment.
    Development,
    /// The staging environment.
    Staging,
    /// The production environment.
    Production,
}

impl Environment {
    /// Retrieves the "active" environment as determined by the `ROCKET_ENV`
    /// environment variable. If `ROCKET_ENV` is not set, returns `Development`.
    ///
    /// # Errors
    ///
    /// Returns a `BadEnv` `ConfigError` if `ROCKET_ENV` contains an invalid
    /// environment name.
    pub fn active() -> Result<Environment, ConfigError> {
        let env_str = env::var(CONFIG_ENV).unwrap_or(Development.to_string());
        env_str.parse().map_err(|_| ConfigError::BadEnv(env_str))
    }

    /// Returns a string with a comma-seperated list of valid environments.
    pub(crate) fn valid() -> &'static str {
        "development, staging, production"
    }

    /// Returns a list of all of the possible environments.
    #[inline]
    pub(crate) fn all() -> [Environment; 3] {
        [Development, Staging, Production]
    }
}

impl FromStr for Environment {
    type Err = ();

    /// Parses a configuration environment from a string. Should be used
    /// indirectly via `str`'s `parse` method.
    ///
    /// # Examples
    ///
    /// Parsing a development environment:
    ///
    /// ```rust
    /// use rocket::config::Environment;
    ///
    /// let env = "development".parse::<Environment>();
    /// assert_eq!(env.unwrap(), Environment::Development);
    ///
    /// let env = "dev".parse::<Environment>();
    /// assert_eq!(env.unwrap(), Environment::Development);
    ///
    /// let env = "devel".parse::<Environment>();
    /// assert_eq!(env.unwrap(), Environment::Development);
    /// ```
    ///
    /// Parsing a staging environment:
    ///
    /// ```rust
    /// use rocket::config::Environment;
    ///
    /// let env = "staging".parse::<Environment>();
    /// assert_eq!(env.unwrap(), Environment::Staging);
    ///
    /// let env = "stage".parse::<Environment>();
    /// assert_eq!(env.unwrap(), Environment::Staging);
    /// ```
    ///
    /// Parsing a production environment:
    ///
    /// ```rust
    /// use rocket::config::Environment;
    ///
    /// let env = "production".parse::<Environment>();
    /// assert_eq!(env.unwrap(), Environment::Production);
    ///
    /// let env = "prod".parse::<Environment>();
    /// assert_eq!(env.unwrap(), Environment::Production);
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let env = match s {
            "dev" | "devel" | "development" => Development,
            "stage" | "staging" => Staging,
            "prod" | "production" => Production,
            _ => return Err(()),
        };

        Ok(env)
    }
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Development => write!(f, "development"),
            Staging => write!(f, "staging"),
            Production => write!(f, "production"),
        }
    }
}
