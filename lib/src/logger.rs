//! Rocket's logging infrastructure.

use std::str::FromStr;
use std::fmt;

use log::{self, Log, LogLevel, LogRecord, LogMetadata};
use yansi::Paint;

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
    #[inline(always)]
    fn max_log_level(&self) -> LogLevel {
        match *self {
            LoggingLevel::Critical => LogLevel::Warn,
            LoggingLevel::Normal => LogLevel::Info,
            LoggingLevel::Debug => LogLevel::Trace,
        }
    }
}

impl FromStr for LoggingLevel {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let level = match s {
            "critical" => LoggingLevel::Critical,
            "normal" => LoggingLevel::Normal,
            "debug" => LoggingLevel::Debug,
            _ => return Err("a log level (debug, normal, critical)")
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

#[doc(hidden)] #[macro_export]
macro_rules! log_ {
    ($name:ident: $format:expr) => { log_!($name: $format,) };
    ($name:ident: $format:expr, $($args:expr),*) => {
        $name!(target: "_", $format, $($args),*);
    };
}

#[doc(hidden)] #[macro_export]
macro_rules! launch_info {
    ($format:expr, $($args:expr),*) => {
        error!(target: "launch", $format, $($args),*)
    }
}

#[doc(hidden)] #[macro_export]
macro_rules! error_ { ($($args:expr),+) => { log_!(error: $($args),+); }; }
#[doc(hidden)] #[macro_export]
macro_rules! info_ { ($($args:expr),+) => { log_!(info: $($args),+); }; }
#[doc(hidden)] #[macro_export]
macro_rules! trace_ { ($($args:expr),+) => { log_!(trace: $($args),+); }; }
#[doc(hidden)] #[macro_export]
macro_rules! debug_ { ($($args:expr),+) => { log_!(debug: $($args),+); }; }
#[doc(hidden)] #[macro_export]
macro_rules! warn_ { ($($args:expr),+) => { log_!(warn: $($args),+); }; }

impl Log for RocketLogger {
    #[inline(always)]
    fn enabled(&self, md: &LogMetadata) -> bool {
        md.level() <= self.0.max_log_level()
    }

    fn log(&self, record: &LogRecord) {
        // Print nothing if this level isn't enabled.
        if !self.enabled(record.metadata()) {
            return;
        }

        // We use the `launch_info` macro to "fake" a high priority info
        // message. We want to print the message unless the user uses a custom
        // drain, so we set it's status to critical, but reset it here to info.
        let level = match record.target() {
            "launch" => Info,
            _ => record.level()
        };

        // Don't print Hyper or Rustls messages unless debug is enabled.
        let from_hyper = record.location().module_path().starts_with("hyper::");
        let from_rustls = record.location().module_path().starts_with("rustls::");
        if self.0 != LoggingLevel::Debug && (from_hyper || from_rustls) {
            return;
        }

        // In Rocket, we abuse target with value "_" to indicate indentation.
        if record.target() == "_" && self.0 != LoggingLevel::Critical {
            print!("    {} ", Paint::white("=>"));
        }

        use log::LogLevel::*;
        match level {
            Info => println!("{}", Paint::blue(record.args())),
            Trace => println!("{}", Paint::purple(record.args())),
            Error => {
                println!("{} {}",
                         Paint::red("Error:").bold(),
                         Paint::red(record.args()))
            }
            Warn => {
                println!("{} {}",
                         Paint::yellow("Warning:").bold(),
                         Paint::yellow(record.args()))
            }
            Debug => {
                let loc = record.location();
                print!("\n{} ", Paint::blue("-->").bold());
                println!("{}:{}", Paint::blue(loc.file()), Paint::blue(loc.line()));
                println!("{}", record.args());
            }
        }
    }
}

#[doc(hidden)]
pub fn try_init(level: LoggingLevel, verbose: bool) {
    if !::isatty::stdout_isatty() {
        Paint::disable();
    } else if cfg!(windows) {
        Paint::enable_windows_ascii();
    }

    let result = log::set_logger(|max_log_level| {
        max_log_level.set(level.max_log_level().to_log_level_filter());
        Box::new(RocketLogger(level))
    });

    if let Err(err) = result {
        if verbose {
            println!("Logger failed to initialize: {}", err);
        }
    }
}

#[doc(hidden)]
pub fn init(level: LoggingLevel) {
    try_init(level, true)
}
