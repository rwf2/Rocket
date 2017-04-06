use std::path::PathBuf;
use std::error::Error;
use std::fmt;

use toml::de::Error as TomlParseError;

use super::Environment;
use self::ConfigError::*;

use term_painter::Color::White;
use term_painter::ToStyle;

// Workaround for `PartialEq`
#[derive(Debug, Clone)]
pub struct ParsingError(TomlParseError);

impl PartialEq for ParsingError {
    fn eq(&self, _other: &ParsingError) -> bool {
        true
    }
}

/// The type of a configuration error.
#[derive(Debug, PartialEq, Clone)]
pub enum ConfigError {
    /// The current working directory could not be determined.
    BadCWD,
    /// The configuration file was not found.
    NotFound,
    /// There was an I/O error while reading the configuration file.
    IOError,
    /// The path at which the configuration file was found was invalid.
    ///
    /// Parameters: (path, reason)
    BadFilePath(PathBuf, &'static str),
    /// An environment specified in `ROCKET_ENV` is invalid.
    ///
    /// Parameters: (environment_name)
    BadEnv(String),
    /// An environment specified as a table `[environment]` is invalid.
    ///
    /// Parameters: (environment_name, filename)
    BadEntry(String, PathBuf),
    /// A config key was specified with a value of the wrong type.
    ///
    /// Parameters: (entry_name, expected_type, actual_type, filename)
    BadType(String, &'static str, &'static str, PathBuf),
    /// There was a TOML parsing error.
    ///
    /// Parameters: (toml_parse_error)
    ParseError(ParsingError),
    /// There was a TOML parsing error in a config environment variable.
    ///
    /// Parameters: (env_key, env_value, expected type)
    BadEnvVal(String, String, &'static str),
}

impl ConfigError {
    /// Prints this configuration error with Rocket formatting.
    pub fn pretty_print(&self) {
        let valid_envs = Environment::valid();
        match *self {
            BadCWD => error!("couldn't get current working directory"),
            NotFound => error!("config file was not found"),
            IOError => error!("failed reading the config file: IO error"),
            BadFilePath(ref path, reason) => {
                error!("configuration file path '{:?}' is invalid", path);
                info_!("{}", reason);
            }
            BadEntry(ref name, ref filename) => {
                let valid_entries = format!("{}, and global", valid_envs);
                error!("[{}] is not a known configuration environment", name);
                info_!("in {:?}", White.paint(filename));
                info_!("valid environments are: {}", White.paint(valid_entries));
            }
            BadEnv(ref name) => {
                error!("'{}' is not a valid ROCKET_ENV value", name);
                info_!("valid environments are: {}", White.paint(valid_envs));
            }
            BadType(ref name, expected, actual, ref filename) => {
                error!("'{}' key could not be parsed", name);
                info_!("in {:?}", White.paint(filename));
                info_!("expected value to be {}, but found {}",
                       White.paint(expected), White.paint(actual));
            }
            ParseError(ref error) => {
                error!("config file could not be parsed as TOML");
                info_!("{}", error.0);
                trace_!("{}", White.paint(&error.0.description()));
            }
            BadEnvVal(ref key, ref value, ref expected) => {
                error!("environment variable '{}={}' could not be parsed",
                       White.paint(key), White.paint(value));
                info_!("value for {:?} must be {}",
                       White.paint(key), White.paint(expected))
            }
        }
    }

    /// Whether this error is of `NotFound` variant.
    #[inline(always)]
    pub fn is_not_found(&self) -> bool {
        match *self {
            NotFound => true,
            _ => false
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            BadCWD => write!(f, "couldn't get current working directory"),
            NotFound => write!(f, "config file was not found"),
            IOError => write!(f, "I/O error while reading the config file"),
            BadFilePath(ref p, _) => write!(f, "{:?} is not a valid config path", p),
            BadEnv(ref e) => write!(f, "{:?} is not a valid `ROCKET_ENV` value", e),
            ParseError(..) => write!(f, "the config file contains invalid TOML"),
            BadEntry(ref e, _) => {
                write!(f, "{:?} is not a valid `[environment]` entry", e)
            }
            BadType(ref n, e, a, _) => {
                write!(f, "type mismatch for '{}'. expected {}, found {}", n, e, a)
            }
            BadEnvVal(ref k, ref v, _) => {
                write!(f, "environment variable '{}={}' could not be parsed", k, v)
            }
        }
    }
}

impl Error for ConfigError {
    fn description(&self) -> &str {
        match *self {
            BadCWD => "the current working directory could not be determined",
            NotFound => "config file was not found",
            IOError => "there was an I/O error while reading the config file",
            BadFilePath(..) => "the config file path is invalid",
            BadEntry(..) => "an environment specified as `[environment]` is invalid",
            BadEnv(..) => "the environment specified in `ROCKET_ENV` is invalid",
            ParseError(..) => "the config file contains invalid TOML",
            BadType(..) => "a key was specified with a value of the wrong type",
            BadEnvVal(..) => "an environment variable could not be parsed",
        }
    }
}

impl From<TomlParseError> for ConfigError {
    fn from(error: TomlParseError) -> Self {
        ParseError(ParsingError(error))
    }
}
