use std::collections::HashSet;

use figment::value::magic::{Either, RelativePathBuf};
use serde::{Deserialize, Serialize};

/// TLS configuration: a certificate chain, a private key, client/server ciphers order preference, 1.2 cipher suites, and 1.3 cipher suites.
///
/// Both `certs` and `key` can be configured as a path or as raw bytes. `certs`
/// must be a DER-encoded X.509 TLS certificate chain, while `key` must be a
/// DER-encoded ASN.1 key in either PKCS#8 or PKCS#1 format.
///
/// When a path is configured in a file source, such as `Rocket.toml`, relative
/// paths are interpreted as being relative to the source file's directory.
///
/// If `prefer_server_ciphers_order` is set to true then client's ciphersuite order is ignored and the top ciphersuite in `v12_ciphers` and `v13_ciphers`
/// which is supported by the client is chosen. Note in that case that v13_ciphers` are prioritized over `v12_ciphers`. Otherwise prefer client order.
///
/// `v12_ciphers` and `v13_ciphers` contains ciphers accepted by the server for TLS 1.2 and TLS 1.3 respectively.
/// Refer to `V12Ciphers` and `V13Ciphers` for supported ciphers.
///
/// The following example illustrates manual configuration:
///
/// ```rust
/// use rocket::Config;
/// use rocket::config::{V12Ciphers, V13Ciphers};
///
/// let figment = rocket::Config::figment()
///     .merge(("tls.certs", "strings/are/paths/certs.pem"))
///     .merge(("tls.key", vec![0; 32]))
///     .merge(("tls.prefer_server_ciphers_order", true))
///     .merge(("tls.v12_ciphers", Vec::<V12Ciphers>::new()))
///     .merge(("tls.v13_ciphers", vec![V13Ciphers::Aes128GcmSha256]));
///
/// let config = rocket::Config::from(figment);
/// let tls_config = config.tls.as_ref().unwrap();
/// assert!(tls_config.certs().is_left());
/// assert!(tls_config.key().is_right());
/// assert_eq!(tls_config.prefer_server_ciphers_order, true);
/// assert_eq!(tls_config.v12_ciphers, vec![]);
/// assert_eq!(tls_config.v13_ciphers, vec![V13Ciphers::Aes128GcmSha256]);
/// ```
///
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[serde(try_from = "NonValidatedTlsConfig")]
pub struct TlsConfig {
    /// Path or raw bytes for the DER-encoded X.509 TLS certificate chain.
    pub(crate) certs: Either<RelativePathBuf, Vec<u8>>,
    /// Path or raw bytes to DER-encoded ASN.1 key in either PKCS#8 or PKCS#1
    /// format.
    pub(crate) key: Either<RelativePathBuf, Vec<u8>>,
    /// Ignore the client order and choose the top ciphersuite in `v12_ciphers` and `v13_ciphers` which is supported by the client
    pub prefer_server_ciphers_order: bool,
    /// Ordered list of all the TLS1.2 cipher suites accepted by server.
    pub v12_ciphers: Vec<V12Ciphers>,
    /// Ordered list of all the TLS1.3 cipher suites accepted by server.
    pub v13_ciphers: Vec<V13Ciphers>,
}

// Shadow type used for validation
#[derive(Deserialize, Serialize)]
struct NonValidatedTlsConfig {
    certs: Either<RelativePathBuf, Vec<u8>>,
    key: Either<RelativePathBuf, Vec<u8>>,
    prefer_server_ciphers_order: bool,
    v12_ciphers: Vec<V12Ciphers>,
    v13_ciphers: Vec<V13Ciphers>,
}

impl std::convert::TryFrom<NonValidatedTlsConfig> for TlsConfig {
    type Error = &'static str;
    fn try_from(non_validated_config: NonValidatedTlsConfig) -> Result<Self, Self::Error> {
        if non_validated_config.v12_ciphers.is_empty() && non_validated_config.v13_ciphers.is_empty() {
            return Err("Both v12 and v13 ciphers are empty: there should be at least 1 cipher enabled");
        }

        if non_validated_config.v12_ciphers.len() != non_validated_config.v12_ciphers.iter().copied().collect::<HashSet<_>>().len() {
            return Err("v12 ciphers should not contain repeated ciphers");
        }

        if non_validated_config.v13_ciphers.len() != non_validated_config.v13_ciphers.iter().copied().collect::<HashSet<_>>().len() {
            return Err("v13 ciphers should not contain repeated ciphers");
        }

        Ok(TlsConfig {
            certs: non_validated_config.certs,
            key: non_validated_config.key,
            prefer_server_ciphers_order: non_validated_config.prefer_server_ciphers_order,
            v12_ciphers: non_validated_config.v12_ciphers,
            v13_ciphers: non_validated_config.v13_ciphers,
        })
    }
}

/// This enum is used in `TlsConfig` to determine which TLS 1.2 cipher suite to accept in the server.
/// It contains only TlS 1.2 cipher suites supported by Rocket.
// This list is based on cipher suites supported by rustls, they can be found at rustls::ALL_CIPHERSUITES
#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy, Deserialize, Serialize)]
pub enum V12Ciphers {
    EcdheEcdsaWithChacha20Poly1305Sha256,
    EcdheRsaWithChacha20Poly1305Sha256,
    EcdheEcdsaWithAes256GcmSha384,
    EcdheEcdsaWithAes128GcmSha256,
    EcdheRsaWithAes256GcmSha384,
    EcdheRsaWithAes128GcmSha256,
}

/// This enum is used in `TlsConfig` to determine which 1.3 cipher suite to accept in the server.
/// It contains only TlS 1.3 cipher suites supported by Rocket.
// This list is based on cipher suites supported by rustls, they can be found at rustls::ALL_CIPHERSUITES
#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy, Deserialize, Serialize)]
pub enum V13Ciphers {
    Chacha20Poly1305Sha256,
    Aes256GcmSha384,
    Aes128GcmSha256,
}

impl TlsConfig {
    /// Constructs a `TlsConfig` from paths to a `certs` certificate-chain
    /// a `key` private-key. This method does no validation; it simply creates a
    /// structure suitable for passing into a [`Config`](crate::Config).
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::TlsConfig;
    /// use rocket::config::V13Ciphers;
    ///
    /// let v12_ciphers = vec![];
    /// let v13_ciphers = vec![V13Ciphers::Aes128GcmSha256, V13Ciphers::Aes256GcmSha384];
    /// let tls_config = TlsConfig::from_paths("/ssl/certs.pem", "/ssl/key.pem", false, v12_ciphers, v13_ciphers);
    /// ```
    ///
    /// # Panics
    /// Panics if both v12_ciphers and v13_ciphers are empty.
    ///
    pub fn from_paths<C, K>(certs: C, key: K, prefer_server_ciphers_order: bool, v12_ciphers: Vec<V12Ciphers>, v13_ciphers: Vec<V13Ciphers>) -> Self
        where C: AsRef<std::path::Path>, K: AsRef<std::path::Path>
    {
        assert!(!v12_ciphers.is_empty() || !v13_ciphers.is_empty());

        TlsConfig {
            certs: Either::Left(certs.as_ref().to_path_buf().into()),
            key: Either::Left(key.as_ref().to_path_buf().into()),
            prefer_server_ciphers_order,
            v12_ciphers,
            v13_ciphers,
        }
    }

    /// Constructs a `TlsConfig` from byte buffers to a `certs`
    /// certificate-chain a `key` private-key. This method does no validation;
    /// it simply creates a structure suitable for passing into a
    /// [`Config`](crate::Config).
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::TlsConfig;
    /// use rocket::config::V13Ciphers;
    ///
    /// let v12_ciphers = vec![];
    /// let v13_ciphers = vec![V13Ciphers::Aes128GcmSha256, V13Ciphers::Aes256GcmSha384];
    /// # let certs_buf = &[];
    /// # let key_buf = &[];
    /// let tls_config = TlsConfig::from_bytes(certs_buf, key_buf, false, v12_ciphers, v13_ciphers);
    /// ```
    ///
    /// # Panics
    /// Panics if both v12_ciphers and v13_ciphers are empty.
    ///
    pub fn from_bytes(certs: &[u8], key: &[u8], prefer_server_ciphers_order: bool, v12_ciphers: Vec<V12Ciphers>, v13_ciphers: Vec<V13Ciphers>) -> Self {
        assert!(!v12_ciphers.is_empty() || !v13_ciphers.is_empty());

        TlsConfig {
            certs: Either::Right(certs.to_vec().into()),
            key: Either::Right(key.to_vec().into()),
            prefer_server_ciphers_order,
            v12_ciphers,
            v13_ciphers,
        }
    }

    /// Returns the value of the `certs` parameter.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Config;
    /// use rocket::config::{V12Ciphers, V13Ciphers};
    ///
    /// let figment = Config::figment()
    ///     .merge(("tls.certs", vec![0; 32]))
    ///     .merge(("tls.key", "/etc/ssl/key.pem"))
    ///     .merge(("tls.prefer_server_ciphers_order", true))
    ///     .merge(("tls.v12_ciphers", Vec::<V12Ciphers>::new()))
    ///     .merge(("tls.v13_ciphers", vec![V13Ciphers::Aes128GcmSha256]));
    ///
    /// let config = rocket::Config::from(figment);
    /// let tls_config = config.tls.as_ref().unwrap();
    /// let cert_bytes = tls_config.certs().right().unwrap();
    /// assert!(cert_bytes.iter().all(|&b| b == 0));
    /// ```
    pub fn certs(&self) -> either::Either<std::path::PathBuf, &[u8]> {
        match &self.certs {
            Either::Left(path) => either::Either::Left(path.relative()),
            Either::Right(bytes) => either::Either::Right(&bytes),
        }
    }

    /// Returns the value of the `key` parameter.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::path::Path;
    /// use rocket::Config;
    /// use rocket::config::{V12Ciphers, V13Ciphers};
    ///
    /// let figment = Config::figment()
    ///     .merge(("tls.certs", vec![0; 32]))
    ///     .merge(("tls.key", "/etc/ssl/key.pem"))
    ///     .merge(("tls.prefer_server_ciphers_order", true))
    ///     .merge(("tls.v12_ciphers", Vec::<V12Ciphers>::new()))
    ///     .merge(("tls.v13_ciphers", vec![V13Ciphers::Aes128GcmSha256]));
    ///
    /// let config = rocket::Config::from(figment);
    /// let tls_config = config.tls.as_ref().unwrap();
    /// let key_path = tls_config.key().left().unwrap();
    /// assert_eq!(key_path, Path::new("/etc/ssl/key.pem"));
    /// ```
    pub fn key(&self) -> either::Either<std::path::PathBuf, &[u8]> {
        match &self.key {
            Either::Left(path) => either::Either::Left(path.relative()),
            Either::Right(bytes) => either::Either::Right(&bytes),
        }
    }
}

#[cfg(feature = "tls")]
type Reader = Box<dyn std::io::BufRead + Sync + Send>;

#[cfg(feature = "tls")]
impl TlsConfig {
    pub(crate) fn to_readers(&self) -> std::io::Result<(Reader, Reader)> {
        use std::{io::{self, Error}, fs};
        use yansi::Paint;

        fn to_reader(value: &Either<RelativePathBuf, Vec<u8>>) -> io::Result<Reader> {
            match value {
                Either::Left(path) => {
                    let path = path.relative();
                    let file = fs::File::open(&path).map_err(move |e| {
                        Error::new(e.kind(), format!("error reading TLS file `{}`: {}",
                                Paint::white(figment::Source::File(path)), e))
                    })?;

                    Ok(Box::new(io::BufReader::new(file)))
                }
                Either::Right(vec) => Ok(Box::new(io::Cursor::new(vec.clone()))),
            }
        }

        Ok((to_reader(&self.certs)?, to_reader(&self.key)?))
    }
}
