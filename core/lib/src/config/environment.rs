use super::ConfigError;

use std::fmt;
use std::str::FromStr;
use std::env;

use self::Environment::*;

pub const CONFIG_ENV: &str = "ROCKET_ENV";

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
    /// List of all of the possible environments.
    crate const ALL: [Environment; 3] = [Development, Staging, Production];

    /// String of all valid environments.
    crate const VALID: &'static str = "development, staging, production";

    /// Retrieves the "active" environment as determined by the `ROCKET_ENV`
    /// environment variable. If `ROCKET_ENV` is not set, returns `Development`
    /// when the application was compiled in `debug` mode and `Production` when
    /// the application was compiled in `release` mode.
    ///
    /// # Errors
    ///
    /// Returns a `BadEnv` `ConfigError` if `ROCKET_ENV` is set and contains an
    /// invalid or unknown environment name.
    pub fn active() -> Result<Environment, ConfigError> {
        match env::var(CONFIG_ENV) {
            Ok(s) => s.parse().map_err(|_| ConfigError::BadEnv(s)),
            #[cfg(debug_assertions)]
            _ => Ok(Development),
            #[cfg(not(debug_assertions))]
            _ => Ok(Production),
        }
    }

    /// Returns `true` if `self` is `Environment::Development`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::Environment;
    ///
    /// assert!(Environment::Development.is_dev());
    /// assert!(!Environment::Production.is_dev());
    /// ```
    #[inline]
    pub fn is_dev(self) -> bool {
        self == Development
    }

    /// Returns `true` if `self` is `Environment::Staging`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::Environment;
    ///
    /// assert!(Environment::Staging.is_stage());
    /// assert!(!Environment::Production.is_stage());
    /// ```
    #[inline]
    pub fn is_stage(self) -> bool {
        self == Staging
    }

    /// Returns `true` if `self` is `Environment::Production`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::Environment;
    ///
    /// assert!(Environment::Production.is_prod());
    /// assert!(!Environment::Staging.is_prod());
    /// ```
    #[inline]
    pub fn is_prod(self) -> bool {
        self == Production
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Development => write!(f, "development"),
            Staging => write!(f, "staging"),
            Production => write!(f, "production"),
        }
    }
}
