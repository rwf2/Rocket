use std::path::PathBuf;

use super::Environment;

use term_painter::Color::White;
use term_painter::ToStyle;

/// The type of a configuration parsing error.
#[derive(Debug, PartialEq, Clone)]
pub struct ParsingError {
    /// Start and end byte indices into the source code where parsing failed.
    pub byte_range: (usize, usize),
    /// The (line, column) in the source code where parsing failure began.
    pub start: (usize, usize),
    /// The (line, column) in the source code where parsing failure ended.
    pub end: (usize, usize),
    /// A description of the parsing error that occured.
    pub desc: String,
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
    /// Parameters: (toml_source_string, filename, error_list)
    ParseError(String, PathBuf, Vec<ParsingError>),
}

impl ConfigError {
    /// Prints this configuration error with Rocket formatting.
    pub fn pretty_print(&self) {
        use self::ConfigError::*;

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
            ParseError(ref source, ref filename, ref errors) => {
                for error in errors {
                    let (lo, hi) = error.byte_range;
                    let (line, col) = error.start;
                    let error_source = &source[lo..hi];

                    error!("config file could not be parsed as TOML");
                    info_!("at {:?}:{}:{}", White.paint(filename), line + 1, col + 1);
                    trace_!("'{}' - {}", error_source, White.paint(&error.desc));
                }
            }
        }
    }

    /// Whether this error is of `NotFound` variant.
    #[inline(always)]
    pub fn is_not_found(&self) -> bool {
        use self::ConfigError::*;
        match *self {
            NotFound => true,
            _ => false
        }
    }
}
