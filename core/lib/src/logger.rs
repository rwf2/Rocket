//! Rocket's logging infrastructure.

use std::{fmt, env};
use std::str::FromStr;

use log;
use yansi::Paint;

crate const COLORS_ENV: &str = "ROCKET_CLI_COLORS";

struct RocketLogger(LoggingLevel);

/// Defines the different levels for log messages.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum LoggingLevel {
    /// Only shows errors, warnings, and launch information.
    Critical,
    /// Shows everything except debug and trace information.
    Normal,
    /// Shows everything.
    Debug,
    /// Shows nothing.
    Off,
}

impl LoggingLevel {
    #[inline(always)]
    fn to_level_filter(self) -> log::LevelFilter {
        match self {
            LoggingLevel::Critical => log::LevelFilter::Warn,
            LoggingLevel::Normal => log::LevelFilter::Info,
            LoggingLevel::Debug => log::LevelFilter::Trace,
            LoggingLevel::Off => log::LevelFilter::Off
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
            "off" => LoggingLevel::Off,
            _ => return Err("a log level (off, debug, normal, critical)")
        };

        Ok(level)
    }
}

impl fmt::Display for LoggingLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let string = match *self {
            LoggingLevel::Critical => "critical",
            LoggingLevel::Normal => "normal",
            LoggingLevel::Debug => "debug",
            LoggingLevel::Off => "off"
        };

        write!(f, "{}", string)
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
        if configged_level != LoggingLevel::Debug && (from_hyper || from_rustls) {
            return;
        }

        // In Rocket, we abuse targets with suffix "_" to indicate indentation.
        let is_launch = record.target().starts_with("launch");
        if record.target().ends_with('_') {
            if configged_level != LoggingLevel::Critical || is_launch {
                print!("    {} ", Paint::default("=>").bold());
            }
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

crate fn try_init(level: LoggingLevel, verbose: bool) -> bool {
    if level == LoggingLevel::Off {
        return false;
    }

    if !atty::is(atty::Stream::Stdout)
        || (cfg!(windows) && !Paint::enable_windows_ascii())
        || env::var_os(COLORS_ENV).map(|v| v == "0" || v == "off").unwrap_or(false)
    {
        Paint::disable();
    }

    push_max_level(level);
    if let Err(e) = log::set_boxed_logger(Box::new(RocketLogger(level))) {
        if verbose {
            eprintln!("Logger failed to initialize: {}", e);
        }

        pop_max_level();
        return false;
    }

    true
}

use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};

static PUSHED: AtomicBool = AtomicBool::new(false);
static LAST_LOG_FILTER: AtomicUsize = AtomicUsize::new(0);

fn filter_to_usize(filter: log::LevelFilter) -> usize {
    match filter {
        log::LevelFilter::Off => 0,
        log::LevelFilter::Error => 1,
        log::LevelFilter::Warn => 2,
        log::LevelFilter::Info => 3,
        log::LevelFilter::Debug => 4,
        log::LevelFilter::Trace => 5,
    }
}

fn usize_to_filter(num: usize) -> log::LevelFilter {
    match num {
        0 => log::LevelFilter::Off,
        1 => log::LevelFilter::Error,
        2 => log::LevelFilter::Warn,
        3 => log::LevelFilter::Info,
        4 => log::LevelFilter::Debug,
        5 => log::LevelFilter::Trace,
        _ => unreachable!("max num is 5 in filter_to_usize")
    }
}

crate fn push_max_level(level: LoggingLevel) {
    LAST_LOG_FILTER.store(filter_to_usize(log::max_level()), Ordering::Release);
    PUSHED.store(true, Ordering::Release);
    log::set_max_level(level.to_level_filter());
}

crate fn pop_max_level() {
    if PUSHED.load(Ordering::Acquire) {
        log::set_max_level(usize_to_filter(LAST_LOG_FILTER.load(Ordering::Acquire)));
    }
}

#[doc(hidden)]
pub fn init(level: LoggingLevel) -> bool {
    try_init(level, true)
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
