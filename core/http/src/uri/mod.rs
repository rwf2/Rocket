//! Types for URIs and traits for rendering URI components.

macro_rules! impl_serde {
    ($T:ty, $expected:literal) => {
        #[cfg(feature = "serde")]
        mod serde {
            use std::fmt;
            use std::marker::PhantomData;
            use super::*;

            use serde_::ser::{Serialize, Serializer};
            use serde_::de::{Deserialize, Deserializer, Error, Visitor};

            impl<'a> Serialize for $T {
                fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                    serializer.serialize_str(&self.to_string())
                }
            }

            struct DeVisitor<'a>(PhantomData<&'a $T>);

            impl<'de, 'a> Visitor<'de> for DeVisitor<'a> {
                type Value = $T;

                fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                    write!(formatter, $expected)
                }

                fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
                    <$T>::parse_owned(v.to_string()).map_err(Error::custom)
                }

                fn visit_string<E: Error>(self, v: String) -> Result<Self::Value, E> {
                    <$T>::parse_owned(v).map_err(Error::custom)
                }
            }

            impl<'a, 'de> Deserialize<'de> for $T {
                fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                    deserializer.deserialize_str(DeVisitor(PhantomData))
                }
            }
        }
    };
}

mod uri;
mod origin;
mod reference;
mod authority;
mod absolute;
mod segments;
mod path_query;
mod asterisk;

pub mod error;
pub mod fmt;

#[doc(inline)]
pub use self::error::Error;

pub use self::uri::*;
pub use self::authority::*;
pub use self::origin::*;
pub use self::absolute::*;
pub use self::segments::*;
pub use self::reference::*;
pub use self::path_query::*;
pub use self::asterisk::*;
