use std::fmt;

use serde::{de, Deserializer, Serialize, Serializer};
use tracing::{level_filters::LevelFilter, Level};

pub fn serialize<S: Serializer>(level: &Option<Level>, s: S) -> Result<S::Ok, S::Error> {
    LevelFilter::from(*level).to_string().serialize(s)
}

pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<Option<Level>, D::Error> {
    struct Visitor;

    const E: &str = r#"one of "off", "error", "warn", "info", "debug", "trace", or 0-5"#;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = Option<Level>;

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "expected {E}")
        }

        fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
            v.try_into()
                .map_err(|_| E::invalid_value(de::Unexpected::Signed(v), &E))
                .and_then(|v| self.visit_u64(v))
        }

        fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
            let filter = match v {
                0 => LevelFilter::OFF,
                1 => LevelFilter::ERROR,
                2 => LevelFilter::WARN,
                3 => LevelFilter::INFO,
                4 => LevelFilter::DEBUG,
                5 => LevelFilter::TRACE,
                _ => return Err(E::invalid_value(de::Unexpected::Unsigned(v), &E)),
            };

            Ok(filter.into_level())
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            v.parse::<LevelFilter>()
                .map(|f| f.into_level())
                .map_err(|_| E::invalid_value(de::Unexpected::Str(v), &E))
        }
    }

    de.deserialize_map(Visitor)
}
