use std::fmt;

use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng, generic_array::GenericArray},
    XChaCha20Poly1305, XNonce,
};
use cookie::Key;
use serde::{de, ser, Deserialize, Serialize};

use crate::request::{Outcome, Request, FromRequest};

const NONCE_LEN: usize = 24; // 192-bit
const KEY_LEN: usize = 32;

#[derive(Debug)]
pub enum Error {
    KeyLengthError,
    NonceFillError,
    EncryptionError,
    DecryptionError,
    EncryptedDataLengthError,
}

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
/// raw bytes.
///
/// ```rust
/// use rocket::config::Config;
///
/// // NOTE: Don't (!) use this key! Generate your own and keep it private!
/// //       e.g. via `head -c64 /dev/urandom | base64`
/// let figment = Config::figment()
///     # .merge(("secret_key", "hPRYyVRiMyxpw5sBB1XeCMN1kFsDCqKvBi2QJxBVHQk="));
///     # /*
///     .merge(("secret_key", "hPrYyЭRiMyµ5sBB1π+CMæ1køFsåqKvBiQJxBVHQk="));
///     # */
///
/// let config = Config::from(figment);
/// assert!(!config.secret_key.is_zero());
/// ```
///
/// When configured in the debug profile with the `secrets` feature enabled, a
/// key set as `0` is automatically regenerated at launch time from the OS's
/// random source if available.
///
/// ```rust
/// use rocket::config::Config;
/// use rocket::local::blocking::Client;
///
/// let figment = Config::figment()
///     .merge(("secret_key", vec![0u8; 64]))
///     .select("debug");
///
/// let rocket = rocket::custom(figment);
/// let client = Client::tracked(rocket).expect("okay in debug");
/// assert!(!client.rocket().config().secret_key.is_zero());
/// ```
///
/// When running in any other profile with the `secrets` feature enabled,
/// providing a key of `0` or not provided a key at all results in an error at
/// launch-time:
///
/// ```rust
/// use rocket::config::Config;
/// use rocket::figment::Profile;
/// use rocket::local::blocking::Client;
/// use rocket::error::ErrorKind;
///
/// let profile = Profile::const_new("staging");
/// let figment = Config::figment()
///     .merge(("secret_key", vec![0u8; 64]))
///     .select(profile.clone());
///
/// let rocket = rocket::custom(figment);
/// let error = Client::tracked(rocket).expect_err("error in non-debug");
/// assert!(matches!(error.kind(), ErrorKind::InsecureSecretKey(profile)));
/// ```
///
/// [private cookies]: https://rocket.rs/master/guide/requests/#private-cookies
/// [configuration guide]: https://rocket.rs/master/guide/configuration/#secret-key
#[derive(Clone)]
#[cfg_attr(nightly, doc(cfg(feature = "secrets")))]
pub struct SecretKey {
    pub(crate) key: Key,
    provided: bool,
}

impl SecretKey {
    /// Returns a secret key that is all zeroes.
    pub(crate) fn zero() -> SecretKey {
        SecretKey { key: Key::from(&[0; 64]), provided: false }
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
        SecretKey { key: Key::from(master), provided: true }
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
        SecretKey { key: Key::derive_from(material), provided: true }
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
        Some(SecretKey { key: Key::try_generate()?, provided: false })
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
        self == &Self::zero()
    }

    /// Returns `true` if `self` was not automatically generated and is not zero.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::SecretKey;
    ///
    /// let master = vec![0u8; 64];
    /// let key = SecretKey::generate().unwrap();
    /// assert!(!key.is_provided());
    ///
    /// let master = vec![0u8; 64];
    /// let key = SecretKey::from(&master);
    /// assert!(!key.is_provided());
    /// ```
    pub fn is_provided(&self) -> bool {
        self.provided && !self.is_zero()
    }

    /// Serialize as `zero` to avoid key leakage.
    pub(crate) fn serialize_zero<S>(&self, ser: S) -> Result<S::Ok, S::Error>
        where S: ser::Serializer
    {
        ser.serialize_bytes(&[0; 32][..])
    }

    /// Encrypts the given data.
    /// Generates a random nonce for each encryption to ensure uniqueness.
    /// Returns the Vec<u8> of the concatenated nonce and ciphertext.
    ///
    /// # Example
    /// ```rust
    /// use rocket::config::SecretKey;
    ///
    /// let plaintext = "I like turtles".as_bytes();
    /// let secret_key = SecretKey::generate().unwrap();
    ///
    /// let encrypted = secret_key.encrypt(&plaintext).unwrap();
    /// let decrypted = secret_key.decrypt(&encrypted).unwrap();
    ///
    /// assert_eq!(decrypted, plaintext);
    /// ```
    pub fn encrypt<T: AsRef<[u8]>>(&self, value: T) -> Result<Vec<u8>, Error> {
        // Convert the encryption key to a fixed-length array
        let key: [u8; KEY_LEN] = self.key
            .encryption()
            .try_into()
            .map_err(|_| Error::KeyLengthError)?;

        let cipher = XChaCha20Poly1305::new(GenericArray::from_slice(&key));
        let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);

        let ciphertext = cipher
            .encrypt(&nonce, value.as_ref())
            .map_err(|_| Error::EncryptionError)?;

        // Prepare a vector to hold the nonce and ciphertext
        let mut encrypted_data = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        encrypted_data.extend_from_slice(nonce.as_slice());
        encrypted_data.extend_from_slice(&ciphertext);

        Ok(encrypted_data)
    }

    /// Decrypts the given encrypted data.
    /// Extracts the nonce from the data and uses it for decryption.
    /// Returns the decrypted Vec<u8>.
    pub fn decrypt<T: AsRef<[u8]>>(&self, encrypted: T) -> Result<Vec<u8>, Error> {
        let encrypted = encrypted.as_ref();

        // Check if the length of decoded data is at least the length of the nonce
        if encrypted.len() <= NONCE_LEN {
            return Err(Error::EncryptedDataLengthError);
        }

        // Split the decoded data into nonce and ciphertext
        let (nonce, ciphertext) = encrypted.split_at(NONCE_LEN);
        let nonce = XNonce::from_slice(nonce);

        // Convert the encryption key to a fixed-length array
        let key: [u8; KEY_LEN] = self.key
            .encryption()
            .try_into()
            .map_err(|_| Error::KeyLengthError)?;

        let cipher = XChaCha20Poly1305::new(GenericArray::from_slice(&key));

        // Decrypt the ciphertext using the nonce
        let decrypted = cipher.decrypt(nonce, ciphertext)
            .map_err(|_| Error::DecryptionError)?;

        Ok(decrypted)
    }
}

impl PartialEq for SecretKey {
    fn eq(&self, other: &Self) -> bool {
        // `Key::partial_eq()` is a constant-time op.
        self.key == other.key
    }
}

#[crate::async_trait]
impl<'r> FromRequest<'r> for &'r SecretKey {
    type Error = std::convert::Infallible;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        Outcome::Success(&req.rocket().config().secret_key)
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

impl fmt::Display for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_zero() {
            f.write_str("[zero]")
        } else {
            match self.provided {
                true => f.write_str("[provided]"),
                false => f.write_str("[generated]"),
            }
        }
    }
}

impl fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as fmt::Display>::fmt(self, f)
    }
}
