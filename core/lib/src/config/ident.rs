use std::fmt;

use serde::{Deserialize, Serialize};
use serde::de::{self, Deserializer};

use crate::http::Header;

/// An identifier (or `None`) to send as the `Server` header.
///
/// # Deserialization
///
/// An `Ident` deserializes from any of the following:
///
/// * `string`
///
///   The string must be a valid `Ident`. See [`Ident::try_new()`] for details.
///
/// * `boolean`
///
///   The boolean must be `false`. The value will be [`Ident::none()`].
///
/// * `Option<string>`
///
///   If `Some`, this is the same as deserializing from the inner string. If
///   `None`, the value is [`Ident::none()`].
///
/// * `unit`
///
///   Always deserializes as [`Ident::none()`].
///
/// # Examples
///
/// As with all Rocket configuration options, when using the default
/// [`Config::figment()`](crate::Config::figment()), `Ident` can be configured
/// via a `Rocket.toml` file. When no value for `ident` is provided, the value
/// defaults to `"Rocket"`. Because a default is provided, configuration only
/// needs to provided to customize or remove the value.
///
/// ```rust
/// # use rocket::figment::{Figment, providers::{Format, Toml}};
/// use rocket::config::{Config, Ident};
///
/// // If these are the contents of `Rocket.toml`...
/// # let toml = Toml::string(r#"
/// [default]
/// ident = false
/// # "#).nested();
///
/// // The config parses as follows:
/// # let config = Config::from(Figment::from(Config::debug_default()).merge(toml));
/// assert_eq!(config.ident, Ident::none());
///
/// // If these are the contents of `Rocket.toml`...
/// # let toml = Toml::string(r#"
/// [default]
/// ident = "My Server"
/// # "#).nested();
///
/// // The config parses as follows:
/// # let config = Config::from(Figment::from(Config::debug_default()).merge(toml));
/// assert_eq!(config.ident, Ident::try_new("My Server").unwrap());
/// ```
///
/// The following example illustrates manual configuration:
///
/// ```rust
/// use rocket::config::{Config, Ident};
///
/// let figment = rocket::Config::figment().merge(("ident", false));
/// let config = rocket::Config::from(figment);
/// assert_eq!(config.ident, Ident::none());
///
/// let figment = rocket::Config::figment().merge(("ident", "Fancy/1.0"));
/// let config = rocket::Config::from(figment);
/// assert_eq!(config.ident, Ident::try_new("Fancy/1.0").unwrap());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Ident(Option<String>);

macro_rules! ident {
    ($value:expr) => {
        {
            #[allow(unknown_lints, eq_op)]
            const _: [(); 0 - !{
                const ASSERT: bool = $crate::http::Header::is_valid_value($value, false);
                ASSERT
            } as usize] = [];

            $crate::config::Ident::try_new($value).unwrap()
        }
    }
}

impl Ident {
    /// Returns a new `Ident` with the string `ident`.
    ///
    /// When configured as the [`Config::ident`](crate::Config::ident), Rocket
    /// will set a `Server` header with the `ident` value on all responses.
    ///
    /// # Errors
    ///
    /// The string `ident` must be non-empty and may only contain visible ASCII
    /// characters. The first character cannot be whitespace. The only
    /// whitespace characters allowed are ` ` (space) and `\t` (horizontal tab).
    /// The string is returned wrapped in `Err` if it contains any invalid
    /// characters.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::Ident;
    ///
    /// let ident = Ident::try_new("Rocket").unwrap();
    /// assert_eq!(ident.as_str(), Some("Rocket"));
    ///
    /// let ident = Ident::try_new("Rocket Run").unwrap();
    /// assert_eq!(ident.as_str(), Some("Rocket Run"));
    ///
    /// let ident = Ident::try_new(" Rocket");
    /// assert!(ident.is_err());
    ///
    /// let ident = Ident::try_new("Rocket\nRun");
    /// assert!(ident.is_err());
    ///
    /// let ident = Ident::try_new("\tShip");
    /// assert!(ident.is_err());
    /// ```
    pub fn try_new<S: Into<String>>(ident: S) -> Result<Ident, String> {
        // This is a little more lenient than reality.
        let ident = ident.into();
        if !Header::is_valid_value(&ident, false) {
            return Err(ident);
        }

        Ok(Ident(Some(ident)))
    }

    /// Returns a new `Ident` which is `None`.
    ///
    /// When configured as the [`Config::ident`](crate::Config::ident), Rocket
    /// will not set a `Server` header on any response. Any `Server` header
    /// emitted by the application will still be written out.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::Ident;
    ///
    /// let ident = Ident::none();
    /// assert_eq!(ident.as_str(), None);
    /// ```
    pub const fn none() -> Ident {
        Ident(None)
    }

    /// Returns `self` as an `Option<&str>`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::Ident;
    ///
    /// let ident = Ident::try_new("Rocket").unwrap();
    /// assert_eq!(ident.as_str(), Some("Rocket"));
    ///
    /// let ident = Ident::try_new("Rocket/1 (Unix)").unwrap();
    /// assert_eq!(ident.as_str(), Some("Rocket/1 (Unix)"));
    ///
    /// let ident = Ident::none();
    /// assert_eq!(ident.as_str(), None);
    /// ```
    pub fn as_str(&self) -> Option<&str> {
        self.0.as_deref()
    }
}

impl<'de> Deserialize<'de> for Ident {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Ident;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a server ident string or `false`")
            }

            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
                if !v {
                    return Ok(Ident::none());
                }

                Err(E::invalid_value(de::Unexpected::Bool(v), &self))
            }

            fn visit_some<D>(self, de: D) -> Result<Self::Value, D::Error>
                where D: Deserializer<'de>
            {
                de.deserialize_string(self)
            }

            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(Ident::none())
            }

            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(Ident::none())
            }

            fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ident::try_new(v)
                    .map_err(|s| E::invalid_value(de::Unexpected::Str(&s), &self))
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                self.visit_string(v.into())
            }
        }

        de.deserialize_string(Visitor)
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_str() {
            Some(name) => name.fmt(f),
            None => "disabled".fmt(f),
        }
    }
}

/// The default `Ident` is `"Rocket"`.
impl Default for Ident {
    fn default() -> Self {
        ident!("Rocket")
    }
}
