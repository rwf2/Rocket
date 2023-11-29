use core::fmt;
use serde::{
    de::{self, Unexpected::{Signed, Str}},
    Deserialize, Serialize
};

/// Configure color output for logging.
#[derive(Clone, Serialize, PartialEq, Debug, Default)]
pub enum CliColors {
    /// Always use colors in logs.
    Always,

    /// Use colors in logs if the terminal supports it.
    #[default]
    Auto,

    /// Never use colors in logs.
    Never
}

impl fmt::Display for CliColors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            CliColors::Always => write!(f, "always"),
            CliColors::Auto => write!(f, "auto"),
            CliColors::Never => write!(f, "never")
        }
    }
}


impl<'de> Deserialize<'de> for CliColors {
    fn deserialize<D: de::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = CliColors;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("0, 1, false, true, always, auto, never")
            }

            fn visit_str<E: de::Error>(self, val: &str) -> Result<CliColors, E> {
                match val.to_lowercase().as_str() {
                    "true" => Ok(CliColors::Auto),
                    "false" => Ok(CliColors::Never),
                    "1" => Ok(CliColors::Auto),
                    "0" => Ok(CliColors::Never),
                    "auto" => Ok(CliColors::Auto),
                    "never" => Ok(CliColors::Never),
                    "always" => Ok(CliColors::Always),
                    _ => Err(E::invalid_value(
                        Str(val),
                        &"0, 1, false, true, always, auto, never",
                    ))
                }
            }

            fn visit_bool<E: de::Error>(self, val: bool) -> Result<CliColors, E> {
                match val {
                    true => Ok(CliColors::Auto),
                    false => Ok(CliColors::Never)
                }
            }

            fn visit_i64<E: de::Error>(self, val: i64) -> Result<CliColors, E> {
                match val {
                    0 => Ok(CliColors::Never),
                    1 => Ok(CliColors::Auto),
                    _ => Err(E::invalid_value(
                        Signed(val),
                        &"0, 1, false, true, always, auto, never",
                    ))
                }
            }
        }

        de.deserialize_any(Visitor)
    }
}
