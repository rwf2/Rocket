//! Rocket's logging infrastructure.

use std::fmt;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{de, Serialize, Serializer, Deserialize, Deserializer};
use yansi::Paint;

/// Reexport the `log` crate as `private`.
pub use log as private;

// Expose logging macros (hidden) for use by core/contrib codegen.
macro_rules! define_log_macro {
    ($name:ident: $kind:ident, $target:expr, $d:tt) => (
        #[doc(hidden)]
        #[macro_export]
        macro_rules! $name {
            ($d ($t:tt)*) => ($crate::log::private::$kind!(target: $target, $d ($t)*))
        }
    );
    ($kind:ident, $indented:ident) => (
        define_log_macro!($kind: $kind, module_path!(), $);
        define_log_macro!($indented: $kind, "_", $);

        pub use $indented;
    )
}

define_log_macro!(error, error_);
define_log_macro!(warn, warn_);
define_log_macro!(info, info_);
define_log_macro!(debug, debug_);
define_log_macro!(trace, trace_);
define_log_macro!(launch_info: warn, "rocket::launch", $);
define_log_macro!(launch_info_: warn, "rocket::launch_", $);

#[derive(Debug)]
struct RocketLogger;

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

pub trait PaintExt {
    fn emoji(item: &str) -> Paint<&str>;
}

// Whether a record is a special `launch_info!` record.
fn is_launch_record(record: &log::Metadata<'_>) -> bool {
    record.target().contains("rocket::launch")
}

impl log::Log for RocketLogger {
    #[inline(always)]
    fn enabled(&self, record: &log::Metadata<'_>) -> bool {
        match log::max_level().to_level() {
            Some(max) => record.level() <= max || is_launch_record(record),
            None => false
        }
    }

    fn log(&self, record: &log::Record<'_>) {
        // Print nothing if this level isn't enabled and this isn't launch info.
        if !self.enabled(record.metadata()) {
            return;
        }

        // Don't print Hyper, Rustls or r2d2 messages unless debug is enabled.
        let max = log::max_level();
        let from = |path| record.module_path().map_or(false, |m| m.starts_with(path));
        let debug_only = from("hyper") || from("rustls") || from("r2d2");
        if log::LevelFilter::from(LogLevel::Debug) > max && debug_only {
            return;
        }

        // In Rocket, we abuse targets with suffix "_" to indicate indentation.
        let indented = record.target().ends_with('_');
        if indented {
            print!("   {} ", Paint::default(">>").bold());
        }

        // Downgrade a physical launch `warn` to logical `info`.
        let level = is_launch_record(record.metadata())
            .then(|| log::Level::Info)
            .unwrap_or_else(|| record.level());

        match level {
            log::Level::Error if !indented => {
                println!("{} {}",
                         Paint::red("Error:").bold(),
                         Paint::red(record.args()).wrap())
            }
            log::Level::Warn if !indented => {
                println!("{} {}",
                         Paint::yellow("Warning:").bold(),
                         Paint::yellow(record.args()).wrap())
            }
            log::Level::Info => println!("{}", Paint::blue(record.args()).wrap()),
            log::Level::Trace => println!("{}", Paint::magenta(record.args()).wrap()),
            log::Level::Warn => println!("{}", Paint::yellow(record.args()).wrap()),
            log::Level::Error => println!("{}", Paint::red(record.args()).wrap()),
            log::Level::Debug => {
                print!("\n{} ", Paint::blue("-->").bold());
                if let Some(file) = record.file() {
                    print!("{}", Paint::blue(file));
                }

                if let Some(line) = record.line() {
                    println!(":{}", Paint::blue(line));
                }

                println!("\t{}", record.args());
            }
        }
    }

    fn flush(&self) {
        // NOOP: We don't buffer any records.
    }
}

pub(crate) fn init_default() {
    crate::log::init(&crate::Config::debug_default())
}

pub(crate) fn init(config: &crate::Config) {
    static ROCKET_LOGGER_SET: AtomicBool = AtomicBool::new(false);

    // Try to initialize Rocket's logger, recording if we succeeded.
    if log::set_boxed_logger(Box::new(RocketLogger)).is_ok() {
        ROCKET_LOGGER_SET.store(true, Ordering::Release);
    }

    // Always disable colors if requested or if they won't work on Windows.
    if !config.cli_colors || !Paint::enable_windows_ascii() {
        Paint::disable();
    }

    // Set Rocket-logger specific settings only if Rocket's logger is set.
    if ROCKET_LOGGER_SET.load(Ordering::Acquire) {
        // Rocket logs to stdout, so disable coloring if it's not a TTY.
        if !atty::is(atty::Stream::Stdout) {
            Paint::disable();
        }

        log::set_max_level(config.log_level.into());
    }
}

impl From<LogLevel> for log::LevelFilter {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Critical => log::LevelFilter::Warn,
            LogLevel::Normal => log::LevelFilter::Info,
            LogLevel::Debug => log::LevelFilter::Trace,
            LogLevel::Off => log::LevelFilter::Off
        }
    }
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

impl PaintExt for Paint<&str> {
    /// Paint::masked(), but hidden on Windows due to broken output. See #1122.
    fn emoji(_item: &str) -> Paint<&str> {
        #[cfg(windows)] { Paint::masked("") }
        #[cfg(not(windows))] { Paint::masked(_item) }
    }
}
