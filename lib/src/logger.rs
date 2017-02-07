//! Rocket's logging infrastructure.

use std::str::FromStr;
use std::fmt;

use slog_term;
use slog::{self, DrainExt};
use slog_scope;

pub use slog::Logger;

pub fn default_for(level: LoggingLevel) -> slog::Logger {
    let drain = slog_term::streamer().stderr().compact().build();
    let drain = slog::LevelFilter::new(drain, level.max_log_level()).fuse();

    slog::Logger::root(drain, slog_o!())
}

/// Defines the different levels for log messages.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum LoggingLevel {
    /// Only shows errors and warning.
    Critical,
    /// Shows everything except debug and trace information.
    Normal,
    /// Shows everything.
    Debug,
}

impl LoggingLevel {
    #[inline(always)]
    fn max_log_level(&self) -> slog::Level {
        match *self {
            LoggingLevel::Critical => slog::Level::Warning,
            LoggingLevel::Normal => slog::Level::Info,
            LoggingLevel::Debug => slog::Level::Trace,
        }
    }
}

impl FromStr for LoggingLevel {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let level = match s {
            "critical" => LoggingLevel::Critical,
            "normal" => LoggingLevel::Normal,
            "debug" => LoggingLevel::Debug,
            _ => return Err(())
        };

        Ok(level)
    }
}

impl fmt::Display for LoggingLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let string = match *self {
            LoggingLevel::Critical => "critical",
            LoggingLevel::Normal => "normal",
            LoggingLevel::Debug => "debug",
        };

        write!(f, "{}", string)
    }
}

#[doc(hidden)]
pub fn init(log: &Logger) {
    slog_scope::set_global_logger(log.clone());
}
