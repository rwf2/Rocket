use std::fmt;
use std::ops::Deref;

use serde::{de, ser, Deserialize, Serialize};

use crate::http::private::cookie::Key;
use crate::request::{Outcome, Request, FromRequest};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum Kind {
    Zero,
    Generated,
    Provided
}

/// A cryptographically secure secret key.
///
/// A `SecretKey` is primarily used by [private cookies]. See the [configuration
/// guide] for further details. It can be configured from 256-bit random
/// material or a 512-bit master key, each as either a base64-encoded string or
/// raw bytes. When compiled in debug mode with the `secrets` feature enabled, a
/// key set a `0` is automatically regenerated from the OS's random source if
/// available.
///
/// ```rust
/// # use rocket::figment::Figment;
/// let figment = Figment::from(rocket::Config::default())
///     .merge(("secret_key", "hPRYyVRiMyxpw5sBB1XeCMN1kFsDCqKvBi2QJxBVHQk="));
///
/// assert!(!rocket::Config::from(figment).secret_key.is_zero());
///
/// let figment = Figment::from(rocket::Config::default())
///     .merge(("secret_key", vec![0u8; 64]));
///
/// # /* as far as I can tell, there's no way to test this properly
/// # https://github.com/rust-lang/cargo/issues/6570
/// # https://github.com/rust-lang/cargo/issues/4737
/// # https://github.com/rust-lang/rust/issues/43031
/// assert!(!rocket::Config::from(figment).secret_key.is_zero());
/// # */
/// ```
///
/// [private cookies]: https://rocket.rs/master/guide/requests/#private-cookies
/// [configuration guide]: https://rocket.rs/master/guide/configuration/#secret-key
#[derive(PartialEq, Clone)]
pub struct SecretKey {
    key: Key,
    kind: Kind,
}

impl SecretKey {
    /// Returns a secret key that is all zeroes.
    pub(crate) fn zero() -> SecretKey {
        SecretKey { key: Key::from(&[0; 64]), kind: Kind::Zero }
    }

    /// Creates a `SecretKey` from a 512-bit `master` key. For security,
    /// `master` _must_ be cryptographically random.
    ///
    /// # Panics
    ///
    /// Panics if `master` < 64 bytes.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::SecretKey;
    ///
    /// # let master = vec![0u8; 64];
    /// let key = SecretKey::from(&master);
    /// ```
    pub fn from(master: &[u8]) -> SecretKey {
        let kind = match master.iter().all(|&b| b == 0) {
            true => Kind::Zero,
            false => Kind::Provided
        };

        SecretKey { key: Key::from(master), kind }
    }

    /// Derives a `SecretKey` from 256 bits of cryptographically random
    /// `material`. For security, `material` _must_ be cryptographically random.
    ///
    /// # Panics
    ///
    /// Panics if `material` < 32 bytes.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::SecretKey;
    ///
    /// # let material = vec![0u8; 32];
    /// let key = SecretKey::derive_from(&material);
    /// ```
    pub fn derive_from(material: &[u8]) -> SecretKey {
        SecretKey { key: Key::derive_from(material), kind: Kind::Provided }
    }

    /// Attempts to generate a `SecretKey` from randomness retrieved from the
    /// OS. If randomness from the OS isn't available, returns `None`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::SecretKey;
    ///
    /// let key = SecretKey::generate();
    /// ```
    pub fn generate() -> Option<SecretKey> {
        Some(SecretKey { key: Key::try_generate()?, kind: Kind::Generated })
    }

    /// Returns `true` if `self` is the `0`-key.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::SecretKey;
    ///
    /// let master = vec![0u8; 64];
    /// let key = SecretKey::from(&master);
    /// assert!(key.is_zero());
    /// ```
    pub fn is_zero(&self) -> bool {
        self.kind == Kind::Zero
    }
}

#[doc(hidden)]
impl Deref for SecretKey {
    type Target = Key;

    fn deref(&self) -> &Self::Target {
        &self.key
    }
}

#[crate::async_trait]
impl<'a, 'r> FromRequest<'a, 'r> for &'a SecretKey {
    type Error = std::convert::Infallible;

    async fn from_request(req: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        Outcome::Success(&req.state.config.secret_key)
    }
}

impl Serialize for SecretKey {
    fn serialize<S: ser::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        // We encode as "zero" to avoid leaking the key.
        ser.serialize_bytes(&[0; 32][..])
    }
}

impl<'de> Deserialize<'de> for SecretKey {
    fn deserialize<D: de::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        use {binascii::{b64decode, hex2bin}, de::Unexpected::Str};

        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = SecretKey;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("256-bit base64 or hex string, or 32-byte slice")
            }

            fn visit_str<E: de::Error>(self, val: &str) -> Result<SecretKey, E> {
                let e = |s| E::invalid_value(Str(s), &"256-bit base64 or hex");

                // `binascii` requires a more space than actual output for padding
                let mut buf = [0u8; 96];
                let bytes = match val.len() {
                    44 | 88 => b64decode(val.as_bytes(), &mut buf).map_err(|_| e(val))?,
                    64 => hex2bin(val.as_bytes(), &mut buf).map_err(|_| e(val))?,
                    n => Err(E::invalid_length(n, &"44 or 88 for base64, 64 for hex"))?
                };

                self.visit_bytes(bytes)
            }

            fn visit_bytes<E: de::Error>(self, bytes: &[u8]) -> Result<SecretKey, E> {
                if bytes.len() < 32 {
                    Err(E::invalid_length(bytes.len(), &"at least 32"))
                } else if bytes.iter().all(|b| *b == 0) {
                    Ok(SecretKey::zero())
                } else if bytes.len() >= 64 {
                    Ok(SecretKey::from(bytes))
                } else {
                    Ok(SecretKey::derive_from(bytes))
                }
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                where A: de::SeqAccess<'de>
            {
                let mut bytes = Vec::with_capacity(seq.size_hint().unwrap_or(0));
                while let Some(byte) = seq.next_element()? {
                    bytes.push(byte);
                }

                self.visit_bytes(&bytes)
            }
        }

        de.deserialize_any(Visitor)
    }
}

impl fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            Kind::Zero => f.write_str("[zero]"),
            Kind::Generated => f.write_str("[generated]"),
            Kind::Provided => f.write_str("[provided]"),
        }
    }
}
