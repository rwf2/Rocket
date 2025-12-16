use std::fmt;

use serde::{de, Deserialize, Serialize};

/// Enable or disable coloring when logging.
///
/// Valid configuration values are:
///
///   * `"always"` - [`CliColors::Always`]
///   * `"auto"`, `1`, or `true` - [`CliColors::Auto`] _(default)_
///   * `"never"`, `0`, or `false` - [`CliColors::Never`]
#[derive(Debug, Copy, Clone, Default, Serialize, PartialEq, Eq, Hash)]
pub enum CliColors {
    /// Always enable colors, irrespective of `stdout` and `stderr`.
    ///
    /// Case-insensitive string values of `"always"` parse as this value.
    Always,

    /// Enable colors _only if_ `stdout` and `stderr` support coloring.
    ///
    /// Case-insensitive string values of `"auto"`, the boolean `true`, and the
    /// integer `1` all parse as this value.
    ///
    /// Only Unix-like systems (Linux, macOS, BSD, etc.), this is equivalent to
    /// checking if `stdout` and `stderr` are both TTYs. On Windows, the console
    /// is queried for ANSI escape sequence based coloring support and enabled
    /// if support is successfully enabled.
    #[default]
    Auto,

    /// Never enable colors, even if `stdout` and `stderr` support them.
    ///
    /// Case-insensitive string values of `"never"`, the boolean `false`, and
    /// the integer `0` all parse as this value.
    Never,
}

impl fmt::Display for CliColors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliColors::Always => write!(f, "always"),
            CliColors::Auto => write!(f, "auto"),
            CliColors::Never => write!(f, "never"),
        }
    }
}

impl<'de> Deserialize<'de> for CliColors {
    fn deserialize<D: de::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = CliColors;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("0, 1, false, true, always, auto, or never")
            }

            fn visit_str<E: de::Error>(self, val: &str) -> Result<CliColors, E> {
                match val.to_lowercase().as_str() {
                    "true" => Ok(CliColors::Auto),
                    "false" => Ok(CliColors::Never),
                    "1" => Ok(CliColors::Auto),
                    "0" => Ok(CliColors::Never),
                    "always" => Ok(CliColors::Always),
                    "auto" => Ok(CliColors::Auto),
                    "never" => Ok(CliColors::Never),
                    _ => Err(E::invalid_value(de::Unexpected::Str(val), &self)),
                }
            }

            fn visit_bool<E: de::Error>(self, val: bool) -> Result<CliColors, E> {
                match val {
                    true => Ok(CliColors::Auto),
                    false => Ok(CliColors::Never),
                }
            }

            fn visit_i64<E: de::Error>(self, val: i64) -> Result<CliColors, E> {
                match val {
                    1 => Ok(CliColors::Auto),
                    0 => Ok(CliColors::Never),
                    _ => Err(E::invalid_value(de::Unexpected::Signed(val), &self)),
                }
            }

            fn visit_u64<E: de::Error>(self, val: u64) -> Result<CliColors, E> {
                match val {
                    1 => Ok(CliColors::Auto),
                    0 => Ok(CliColors::Never),
                    _ => Err(E::invalid_value(de::Unexpected::Unsigned(val), &self)),
                }
            }
        }

        de.deserialize_any(Visitor)
    }
}
