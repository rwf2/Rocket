use std::fmt;

use serde::de;

use crate::http::Header;
use crate::http::uncased::Uncased;

pub(crate) fn deserialize<'de, D>(de: D) -> Result<Option<Uncased<'static>>, D::Error>
    where D: de::Deserializer<'de>
{
    struct Visitor;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = Option<Uncased<'static>>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a valid header name or `false`")
        }

        fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
            if !v {
                return Ok(None);
            }

            Err(E::invalid_value(de::Unexpected::Bool(v), &self))
        }

        fn visit_some<D>(self, de: D) -> Result<Self::Value, D::Error>
            where D: de::Deserializer<'de>
        {
            de.deserialize_string(self)
        }

        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            self.visit_string(v.into())
        }

        fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
            if Header::is_valid_name(&v) {
                Ok(Some(Uncased::from_owned(v)))
            } else {
                Err(E::invalid_value(de::Unexpected::Str(&v), &self))
            }
        }
    }

    de.deserialize_string(Visitor)
}
