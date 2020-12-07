//! Rocket's logging infrastructure.

use std::fmt;
use std::str::FromStr;

use yansi::Paint;
use serde::{de, Serialize, Serializer, Deserialize, Deserializer};

#[derive(Debug)]
struct RocketLogger(LogLevel);

/// Defines the maximum level of log messages to show.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum LogLevel {
    /// Only shows errors and warnings: `"critical"`.
    Critical,
    /// Shows everything except debug and trace information: `"normal"`.
    Normal,
    /// Shows everything: `"debug"`.
    Debug,
    /// Shows nothing: "`"off"`".
    Off,
}

impl LogLevel {
    fn as_str(&self) -> &str {
        match self {
            LogLevel::Critical => "critical",
            LogLevel::Normal => "normal",
            LogLevel::Debug => "debug",
            LogLevel::Off => "off",
        }
    }

    #[inline(always)]
    fn to_level_filter(self) -> log::LevelFilter {
        match self {
            LogLevel::Critical => log::LevelFilter::Warn,
            LogLevel::Normal => log::LevelFilter::Info,
            LogLevel::Debug => log::LevelFilter::Trace,
            LogLevel::Off => log::LevelFilter::Off
        }
    }
}

impl FromStr for LogLevel {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let level = match &*s.to_ascii_lowercase() {
            "critical" => LogLevel::Critical,
            "normal" => LogLevel::Normal,
            "debug" => LogLevel::Debug,
            "off" => LogLevel::Off,
            _ => return Err("a log level (off, debug, normal, critical)")
        };

        Ok(level)
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Serialize for LogLevel {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for LogLevel {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let string = String::deserialize(de)?;
        LogLevel::from_str(&string).map_err(|_| de::Error::invalid_value(
            de::Unexpected::Str(&string),
            &figment::error::OneOf( &["critical", "normal", "debug", "off"])
        ))
    }
}

#[doc(hidden)] #[macro_export]
macro_rules! log_ { ($name:ident: $($args:tt)*) => { $name!(target: "_", $($args)*) }; }
#[doc(hidden)] #[macro_export]
macro_rules! launch_info { ($($args:tt)*) => { info!(target: "launch", $($args)*) } }
#[doc(hidden)] #[macro_export]
macro_rules! launch_info_ { ($($args:tt)*) => { info!(target: "launch_", $($args)*) } }
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

impl log::Log for RocketLogger {
    #[inline(always)]
    fn enabled(&self, record: &log::Metadata<'_>) -> bool {
        match self.0.to_level_filter().to_level() {
            Some(max) => record.level() <= max || record.target().starts_with("launch"),
            None => false
        }
    }

    fn log(&self, record: &log::Record<'_>) {
        // Print nothing if this level isn't enabled and this isn't launch info.
        if !self.enabled(record.metadata()) {
            return;
        }

        // Don't print Hyper or Rustls messages unless debug is enabled.
        let configged_level = self.0;
        let from_hyper = record.module_path().map_or(false, |m| m.starts_with("hyper::"));
        let from_rustls = record.module_path().map_or(false, |m| m.starts_with("rustls::"));
        if configged_level != LogLevel::Debug && (from_hyper || from_rustls) {
            return;
        }

        // In Rocket, we abuse targets with suffix "_" to indicate indentation.
        let is_launch = record.target().starts_with("launch");
        if record.target().ends_with('_') && (configged_level != LogLevel::Critical || is_launch) {
            print!("    {} ", Paint::default("=>").bold());
        }

        match record.level() {
            log::Level::Info => println!("{}", Paint::blue(record.args()).wrap()),
            log::Level::Trace => println!("{}", Paint::magenta(record.args()).wrap()),
            log::Level::Error => {
                println!("{} {}",
                         Paint::red("Error:").bold(),
                         Paint::red(record.args()).wrap())
            }
            log::Level::Warn => {
                println!("{} {}",
                         Paint::yellow("Warning:").bold(),
                         Paint::yellow(record.args()).wrap())
            }
            log::Level::Debug => {
                print!("\n{} ", Paint::blue("-->").bold());
                if let Some(file) = record.file() {
                    print!("{}", Paint::blue(file));
                }

                if let Some(line) = record.line() {
                    println!(":{}", Paint::blue(line));
                }

                println!("{}", record.args());
            }
        }
    }

    fn flush(&self) {
        // NOOP: We don't buffer any records.
    }
}

pub(crate) fn try_init(level: LogLevel, colors: bool, verbose: bool) -> bool {
    if level == LogLevel::Off {
        return false;
    }

    if !atty::is(atty::Stream::Stdout)
        || (cfg!(windows) && !Paint::enable_windows_ascii())
        || !colors
    {
        Paint::disable();
    }

    if let Err(e) = log::set_boxed_logger(Box::new(RocketLogger(level))) {
        if verbose {
            eprintln!("Logger failed to initialize: {}", e);
        }

        return false;
    }

    log::set_max_level(level.to_level_filter());
    true
}

pub trait PaintExt {
    fn emoji(item: &str) -> Paint<&str>;
}

impl PaintExt for Paint<&str> {
    /// Paint::masked(), but hidden on Windows due to broken output. See #1122.
    fn emoji(_item: &str) -> Paint<&str> {
        #[cfg(windows)] { Paint::masked("") }
        #[cfg(not(windows))] { Paint::masked(_item) }
    }
}

#[doc(hidden)]
pub fn init(level: LogLevel) -> bool {
    try_init(level, true, true)
}

// Expose logging macros as (hidden) funcions for use by core/contrib codegen.
macro_rules! external_log_function {
    ($fn_name:ident: $macro_name:ident) => (
        #[doc(hidden)] #[inline(always)]
        pub fn $fn_name(msg: &str) { $macro_name!("{}", msg); }
    )
}

external_log_function!(error: error);
external_log_function!(error_: error_);
external_log_function!(warn: warn);
external_log_function!(warn_: warn_);
