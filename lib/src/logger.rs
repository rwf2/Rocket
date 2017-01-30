//! Rocket's logging infrastructure.

use std::str::FromStr;
use std::fmt;

// use log::{self, Log, LogLevel, LogRecord, LogMetadata};
use slog_term;
use slog::{self, DrainExt};
use slog_scope;

use term_painter::Color::*;
use term_painter::ToStyle;

struct RocketLogger(LoggingLevel);

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
    // #[inline(always)]
    // fn max_log_level(&self) -> LogLevel {
    //     match *self {
    //         LoggingLevel::Critical => LogLevel::Warn,
    //         LoggingLevel::Normal => LogLevel::Info,
    //         LoggingLevel::Debug => LogLevel::Trace,
    //     }
    // }
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
pub fn init(level: LoggingLevel) {
    // TODO: Configure drain in a far more complex fashion
    let drain = slog_term::streamer().stderr().full().build().fuse();
    let root_log = slog::Logger::root(drain, slog_o!());

    // Set _global_ scope of log/info/etc. macros
    slog_scope::set_global_logger(root_log);
}
