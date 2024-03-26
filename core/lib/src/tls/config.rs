use std::io;

use rustls::crypto::{ring, CryptoProvider};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use figment::value::magic::{Either, RelativePathBuf};
use serde::{Deserialize, Serialize};
use indexmap::IndexSet;

use crate::tls::error::{Result, Error, KeyError};

/// TLS configuration: certificate chain, key, and ciphersuites.
///
/// Four parameters control `tls` configuration:
///
///   * `certs`, `key`
///
///     Both `certs` and `key` can be configured as a path or as raw bytes.
///     `certs` must be a DER-encoded X.509 TLS certificate chain, while `key`
///     must be a DER-encoded ASN.1 key in either PKCS#8, PKCS#1, or SEC1
///     format. When a path is configured in a file, such as `Rocket.toml`,
///     relative paths are interpreted as relative to the source file's
///     directory.
///
///   * `ciphers`
///
///     A list of supported [`CipherSuite`]s in server-preferred order, from
///     most to least. It is not required and defaults to
///     [`CipherSuite::DEFAULT_SET`], the recommended setting.
///
///   * `prefer_server_cipher_order`
///
///     A boolean that indicates whether the server should regard its own
///     ciphersuite preferences over the client's. The default and recommended
///     value is `false`.
///
/// Additionally, the `mutual` parameter controls if and how the server
/// authenticates clients via mutual TLS. It works in concert with the
/// [`mtls`](crate::mtls) module. See [`MtlsConfig`] for configuration details.
///
/// In `Rocket.toml`, configuration might look like:
///
/// ```toml
/// [default.tls]
/// certs = "private/rsa_sha256_cert.pem"
/// key = "private/rsa_sha256_key.pem"
/// ```
///
/// With a custom programmatic configuration, this might look like:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// use rocket::tls::{TlsConfig, CipherSuite};
/// use rocket::figment::providers::Serialized;
///
/// #[launch]
/// fn rocket() -> _ {
///     let tls = TlsConfig::from_paths("/ssl/certs.pem", "/ssl/key.pem")
///         .with_ciphers(CipherSuite::TLS_V13_SET)
///         .with_preferred_server_cipher_order(true);
///
///     rocket::custom(rocket::Config::figment().merge(("tls", tls)))
/// }
/// ```
///
/// Or by creating a custom figment:
///
/// ```rust
/// use rocket::figment::Figment;
/// use rocket::tls::TlsConfig;
///
/// let figment = Figment::new()
///     .merge(("certs", "path/to/certs.pem"))
///     .merge(("key", vec![0; 32]));
/// #
/// # let tls_config: TlsConfig = figment.extract().unwrap();
/// # assert!(tls_config.certs().is_left());
/// # assert!(tls_config.key().is_right());
/// # assert_eq!(tls_config.ciphers().count(), 9);
/// # assert!(!tls_config.prefer_server_cipher_order());
/// ```
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
pub struct TlsConfig {
    /// Path to a PEM file with, or raw bytes for, a DER-encoded X.509 TLS
    /// certificate chain.
    pub(crate) certs: Either<RelativePathBuf, Vec<u8>>,
    /// Path to a PEM file with, or raw bytes for, DER-encoded private key in
    /// either PKCS#8 or PKCS#1 format.
    pub(crate) key: Either<RelativePathBuf, Vec<u8>>,
    /// List of TLS cipher suites in server-preferred order.
    #[serde(default = "CipherSuite::default_set")]
    pub(crate) ciphers: IndexSet<CipherSuite>,
    /// Whether to prefer the server's cipher suite order over the client's.
    #[serde(default)]
    pub(crate) prefer_server_cipher_order: bool,
    /// Configuration for mutual TLS, if any.
    #[serde(default)]
    #[cfg(feature = "mtls")]
    #[cfg_attr(nightly, doc(cfg(feature = "mtls")))]
    pub(crate) mutual: Option<crate::mtls::MtlsConfig>,
}

/// A supported TLS cipher suite.
#[allow(non_camel_case_types)]
#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash, Deserialize, Serialize)]
#[non_exhaustive]
pub enum CipherSuite {
    /// The TLS 1.3 `TLS_CHACHA20_POLY1305_SHA256` cipher suite.
    TLS_CHACHA20_POLY1305_SHA256,
    /// The TLS 1.3 `TLS_AES_256_GCM_SHA384` cipher suite.
    TLS_AES_256_GCM_SHA384,
    /// The TLS 1.3 `TLS_AES_128_GCM_SHA256` cipher suite.
    TLS_AES_128_GCM_SHA256,

    /// The TLS 1.2 `TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256` cipher suite.
    TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256,
    /// The TLS 1.2 `TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256` cipher suite.
    TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
    /// The TLS 1.2 `TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384` cipher suite.
    TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
    /// The TLS 1.2 `TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256` cipher suite.
    TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256,
    /// The TLS 1.2 `TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384` cipher suite.
    TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
    /// The TLS 1.2 `TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256` cipher suite.
    TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
}

impl Default for TlsConfig {
    fn default() -> Self {
        TlsConfig {
            certs: Either::Right(vec![]),
            key: Either::Right(vec![]),
            ciphers: CipherSuite::default_set(),
            prefer_server_cipher_order: false,
            #[cfg(feature = "mtls")]
            mutual: None,
        }
    }
}

impl TlsConfig {
    /// Constructs a `TlsConfig` from paths to a `certs` certificate chain
    /// a `key` private-key. This method does no validation; it simply creates a
    /// structure suitable for passing into a [`Config`](crate::Config).
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::tls::TlsConfig;
    ///
    /// let tls_config = TlsConfig::from_paths("/ssl/certs.pem", "/ssl/key.pem");
    /// ```
    pub fn from_paths<C, K>(certs: C, key: K) -> Self
        where C: AsRef<std::path::Path>, K: AsRef<std::path::Path>,
    {
        TlsConfig {
            certs: Either::Left(certs.as_ref().to_path_buf().into()),
            key: Either::Left(key.as_ref().to_path_buf().into()),
            ..TlsConfig::default()
        }
    }

    /// Constructs a `TlsConfig` from byte buffers to a `certs`
    /// certificate chain a `key` private-key. This method does no validation;
    /// it simply creates a structure suitable for passing into a
    /// [`Config`](crate::Config).
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::tls::TlsConfig;
    ///
    /// # let certs_buf = &[];
    /// # let key_buf = &[];
    /// let tls_config = TlsConfig::from_bytes(certs_buf, key_buf);
    /// ```
    pub fn from_bytes(certs: &[u8], key: &[u8]) -> Self {
        TlsConfig {
            certs: Either::Right(certs.to_vec()),
            key: Either::Right(key.to_vec()),
            ..TlsConfig::default()
        }
    }

    /// Sets the cipher suites supported by the server and their order of
    /// preference from most to least preferred.
    ///
    /// If a suite appears more than once in `ciphers`, only the first suite
    /// (and its relative order) is considered. If all cipher suites for a
    /// version oF TLS are disabled, the respective protocol itself is disabled
    /// entirely.
    ///
    /// # Example
    ///
    /// Disable TLS v1.2 by selecting only TLS v1.3 cipher suites:
    ///
    /// ```rust
    /// use rocket::tls::{TlsConfig, CipherSuite};
    ///
    /// # let certs_buf = &[];
    /// # let key_buf = &[];
    /// let tls_config = TlsConfig::from_bytes(certs_buf, key_buf)
    ///     .with_ciphers(CipherSuite::TLS_V13_SET);
    /// ```
    ///
    /// Enable only ChaCha20-Poly1305 based TLS v1.2 and TLS v1.3 cipher suites:
    ///
    /// ```rust
    /// use rocket::tls::{TlsConfig, CipherSuite};
    ///
    /// # let certs_buf = &[];
    /// # let key_buf = &[];
    /// let tls_config = TlsConfig::from_bytes(certs_buf, key_buf)
    ///     .with_ciphers([
    ///         CipherSuite::TLS_CHACHA20_POLY1305_SHA256,
    ///         CipherSuite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256,
    ///         CipherSuite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
    ///     ]);
    /// ```
    ///
    /// Later duplicates are ignored.
    ///
    /// ```rust
    /// use rocket::tls::{TlsConfig, CipherSuite};
    ///
    /// # let certs_buf = &[];
    /// # let key_buf = &[];
    /// let tls_config = TlsConfig::from_bytes(certs_buf, key_buf)
    ///     .with_ciphers([
    ///         CipherSuite::TLS_CHACHA20_POLY1305_SHA256,
    ///         CipherSuite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256,
    ///         CipherSuite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256,
    ///         CipherSuite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
    ///         CipherSuite::TLS_CHACHA20_POLY1305_SHA256,
    ///     ]);
    ///
    /// let ciphers: Vec<_> = tls_config.ciphers().collect();
    /// assert_eq!(ciphers, &[
    ///     CipherSuite::TLS_CHACHA20_POLY1305_SHA256,
    ///     CipherSuite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256,
    ///     CipherSuite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
    /// ]);
    /// ```
    pub fn with_ciphers<C>(mut self, ciphers: C) -> Self
        where C: IntoIterator<Item = CipherSuite>
    {
        self.ciphers = ciphers.into_iter().collect();
        self
    }

    /// Whether to prefer the server's cipher suite order and ignore the
    /// client's preferences (`true`) or choose the first supported ciphersuite
    /// in the client's preference list (`false`). The default prefer's the
    /// client's order (`false`).
    ///
    /// During TLS cipher suite negotiation, the client presents a set of
    /// supported ciphers in its preferred order. From this list, the server
    /// chooses one cipher suite. By default, the server chooses the first
    /// cipher it supports from the list.
    ///
    /// By setting `prefer_server_order` to `true`, the server instead chooses
    /// the first ciphersuite in it prefers that the client also supports,
    /// ignoring the client's preferences.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::tls::{TlsConfig, CipherSuite};
    ///
    /// # let certs_buf = &[];
    /// # let key_buf = &[];
    /// let tls_config = TlsConfig::from_bytes(certs_buf, key_buf)
    ///     .with_ciphers(CipherSuite::TLS_V13_SET)
    ///     .with_preferred_server_cipher_order(true);
    /// ```
    pub fn with_preferred_server_cipher_order(mut self, prefer_server_order: bool) -> Self {
        self.prefer_server_cipher_order = prefer_server_order;
        self
    }

    /// Set mutual TLS configuration. See
    /// [`MtlsConfig`](crate::mtls::MtlsConfig) for details.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::tls::TlsConfig;
    /// use rocket::mtls::MtlsConfig;
    ///
    /// # let certs = &[];
    /// # let key = &[];
    /// let mtls_config = MtlsConfig::from_path("path/to/cert.pem").mandatory(true);
    /// let tls_config = TlsConfig::from_bytes(certs, key).with_mutual(mtls_config);
    /// assert!(tls_config.mutual().is_some());
    /// ```
    #[cfg(feature = "mtls")]
    #[cfg_attr(nightly, doc(cfg(feature = "mtls")))]
    pub fn with_mutual(mut self, config: crate::mtls::MtlsConfig) -> Self {
        self.mutual = Some(config);
        self
    }

    /// Returns the value of the `certs` parameter.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::path::Path;
    /// use rocket::tls::TlsConfig;
    /// use rocket::figment::Figment;
    ///
    /// let figment = Figment::new()
    ///     .merge(("certs", "/path/to/certs.pem"))
    ///     .merge(("key", vec![0; 32]));
    ///
    /// let tls_config: TlsConfig = figment.extract().unwrap();
    /// let cert_path = tls_config.certs().left().unwrap();
    /// assert_eq!(cert_path, Path::new("/path/to/certs.pem"));
    /// ```
    pub fn certs(&self) -> either::Either<std::path::PathBuf, &[u8]> {
        match &self.certs {
            Either::Left(path) => either::Either::Left(path.relative()),
            Either::Right(bytes) => either::Either::Right(bytes),
        }
    }

    pub fn certs_reader(&self) -> io::Result<Box<dyn io::BufRead + Sync + Send>> {
        to_reader(&self.certs)
    }

    /// Returns the value of the `key` parameter.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::path::Path;
    /// use rocket::tls::TlsConfig;
    /// use rocket::figment::Figment;
    ///
    /// let figment = Figment::new()
    ///     .merge(("certs", vec![0; 32]))
    ///     .merge(("key", "/etc/ssl/key.pem"));
    ///
    /// let tls_config: TlsConfig = figment.extract().unwrap();
    /// let key_path = tls_config.key().left().unwrap();
    /// assert_eq!(key_path, Path::new("/etc/ssl/key.pem"));
    /// ```
    pub fn key(&self) -> either::Either<std::path::PathBuf, &[u8]> {
        match &self.key {
            Either::Left(path) => either::Either::Left(path.relative()),
            Either::Right(bytes) => either::Either::Right(bytes),
        }
    }

    pub fn key_reader(&self) -> io::Result<Box<dyn io::BufRead + Sync + Send>> {
        to_reader(&self.key)
    }

    /// Returns an iterator over the enabled cipher suites in their order of
    /// preference from most to least preferred.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::tls::{TlsConfig, CipherSuite};
    ///
    /// # let certs_buf = &[];
    /// # let key_buf = &[];
    /// // The default set is CipherSuite::DEFAULT_SET.
    /// let tls_config = TlsConfig::from_bytes(certs_buf, key_buf);
    /// assert_eq!(tls_config.ciphers().count(), 9);
    /// assert!(tls_config.ciphers().eq(CipherSuite::DEFAULT_SET.iter().copied()));
    ///
    /// // Enable only the TLS v1.3 ciphers.
    /// let tls_v13_config = TlsConfig::from_bytes(certs_buf, key_buf)
    ///     .with_ciphers(CipherSuite::TLS_V13_SET);
    ///
    /// assert_eq!(tls_v13_config.ciphers().count(), 3);
    /// assert!(tls_v13_config.ciphers().eq(CipherSuite::TLS_V13_SET.iter().copied()));
    /// ```
    pub fn ciphers(&self) -> impl Iterator<Item = CipherSuite> + '_ {
        self.ciphers.iter().copied()
    }

    /// Whether the server's cipher suite ordering is preferred or not.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::tls::TlsConfig;
    ///
    /// # let certs_buf = &[];
    /// # let key_buf = &[];
    /// // The default prefers the server's order.
    /// let tls_config = TlsConfig::from_bytes(certs_buf, key_buf);
    /// assert!(!tls_config.prefer_server_cipher_order());
    ///
    /// // Which can be overridden with the eponymous builder method.
    /// let tls_config = TlsConfig::from_bytes(certs_buf, key_buf)
    ///     .with_preferred_server_cipher_order(true);
    ///
    /// assert!(tls_config.prefer_server_cipher_order());
    /// ```
    pub fn prefer_server_cipher_order(&self) -> bool {
        self.prefer_server_cipher_order
    }

    /// Returns the value of the `mutual` parameter.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::path::Path;
    ///
    /// use rocket::tls::TlsConfig;
    /// use rocket::mtls::MtlsConfig;
    ///
    /// # let certs = &[];
    /// # let key = &[];
    /// let mtls_config = MtlsConfig::from_path("path/to/cert.pem").mandatory(true);
    /// let tls_config = TlsConfig::from_bytes(certs, key).with_mutual(mtls_config);
    ///
    /// let mtls = tls_config.mutual().unwrap();
    /// assert_eq!(mtls.ca_certs().unwrap_left(), Path::new("path/to/cert.pem"));
    /// assert!(mtls.mandatory);
    /// ```
    #[cfg(feature = "mtls")]
    #[cfg_attr(nightly, doc(cfg(feature = "mtls")))]
    pub fn mutual(&self) -> Option<&crate::mtls::MtlsConfig> {
        self.mutual.as_ref()
    }

    pub fn validate(&self) -> Result<(), crate::tls::Error> {
        self.server_config().map(|_| ())
    }
}

/// Loads certificates from `reader`.
impl TlsConfig {
    pub(crate) fn load_certs(&self) -> Result<Vec<CertificateDer<'static>>> {
        rustls_pemfile::certs(&mut self.certs_reader()?)
            .collect::<Result<_, _>>()
            .map_err(Error::CertChain)
    }

    /// Load and decode the private key  from `reader`.
    pub(crate) fn load_key(&self) -> Result<PrivateKeyDer<'static>> {
        use rustls_pemfile::Item::*;

        let mut keys = rustls_pemfile::read_all(&mut self.key_reader()?)
            .map(|result| result.map_err(KeyError::Io)
                .and_then(|item| match item {
                    Pkcs1Key(key) => Ok(key.into()),
                    Pkcs8Key(key) => Ok(key.into()),
                    Sec1Key(key) => Ok(key.into()),
                    _ => Err(KeyError::BadItem(item))
                })
            )
            .collect::<Result<Vec<PrivateKeyDer<'static>>, _>>()?;

        if keys.len() != 1 {
            return Err(KeyError::BadKeyCount(keys.len()).into());
        }

        // Ensure we can use the key.
        let key = keys.remove(0);
        self.default_crypto_provider()
            .key_provider
            .load_private_key(key.clone_key())
            .map_err(KeyError::Unsupported)?;

        Ok(key)
    }

    pub(crate) fn default_crypto_provider(&self) -> CryptoProvider {
        CryptoProvider::get_default()
            .map(|arc| (**arc).clone())
            .unwrap_or_else(|| rustls::crypto::CryptoProvider {
                cipher_suites: self.ciphers().map(|cipher| match cipher {
                    CipherSuite::TLS_CHACHA20_POLY1305_SHA256 =>
                        ring::cipher_suite::TLS13_CHACHA20_POLY1305_SHA256,
                    CipherSuite::TLS_AES_256_GCM_SHA384 =>
                        ring::cipher_suite::TLS13_AES_256_GCM_SHA384,
                    CipherSuite::TLS_AES_128_GCM_SHA256 =>
                        ring::cipher_suite::TLS13_AES_128_GCM_SHA256,
                    CipherSuite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256 =>
                        ring::cipher_suite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256,
                    CipherSuite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256 =>
                        ring::cipher_suite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
                    CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384 =>
                        ring::cipher_suite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
                    CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256 =>
                        ring::cipher_suite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256,
                    CipherSuite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384 =>
                        ring::cipher_suite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
                    CipherSuite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256 =>
                        ring::cipher_suite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
                }).collect(),
                ..ring::default_provider()
            })
    }
}

impl CipherSuite {
    /// The default set and order of cipher suites. These are all of the
    /// variants in [`CipherSuite`] in their declaration order.
    pub const DEFAULT_SET: [CipherSuite; 9] = [
        // TLS v1.3 suites...
        CipherSuite::TLS_CHACHA20_POLY1305_SHA256,
        CipherSuite::TLS_AES_256_GCM_SHA384,
        CipherSuite::TLS_AES_128_GCM_SHA256,

        // TLS v1.2 suites...
        CipherSuite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256,
        CipherSuite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
        CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
        CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256,
        CipherSuite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
        CipherSuite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
    ];

    /// The default set and order of cipher suites. These are the TLS 1.3
    /// variants in [`CipherSuite`] in their declaration order.
    pub const TLS_V13_SET: [CipherSuite; 3] = [
        CipherSuite::TLS_CHACHA20_POLY1305_SHA256,
        CipherSuite::TLS_AES_256_GCM_SHA384,
        CipherSuite::TLS_AES_128_GCM_SHA256,
    ];

    /// The default set and order of cipher suites. These are the TLS 1.2
    /// variants in [`CipherSuite`] in their declaration order.
    pub const TLS_V12_SET: [CipherSuite; 6] = [
        CipherSuite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256,
        CipherSuite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
        CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
        CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256,
        CipherSuite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
        CipherSuite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
    ];

    /// Used as the `serde` default for `ciphers`.
    fn default_set() -> IndexSet<Self> {
        Self::DEFAULT_SET.iter().copied().collect()
    }
}

pub(crate) fn to_reader(
    value: &Either<RelativePathBuf, Vec<u8>>
) -> io::Result<Box<dyn io::BufRead + Sync + Send>> {
    match value {
        Either::Left(path) => {
            let path = path.relative();
            let file = std::fs::File::open(&path)
                .map_err(move |e| {
                    let source = figment::Source::File(path);
                    let msg = format!("error reading TLS file `{source}`: {e}");
                    io::Error::new(e.kind(), msg)
                })?;

            Ok(Box::new(io::BufReader::new(file)))
        }
        Either::Right(vec) => Ok(Box::new(io::Cursor::new(vec.clone()))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use figment::{Figment, providers::{Toml, Format}};

    #[test]
    fn test_tls_config_from_file() {
        use crate::tls::{TlsConfig, CipherSuite};
        use pretty_assertions::assert_eq;

        figment::Jail::expect_with(|jail| {
            jail.create_file("Rocket.toml", r#"
                [global]
                shutdown.ctrlc = 0
                ident = false

                [global.tls]
                certs = "/ssl/cert.pem"
                key = "/ssl/key.pem"

                [global.limits]
                forms = "1mib"
                json = "10mib"
                stream = "50kib"
            "#)?;

            let config: TlsConfig = crate::Config::figment().extract_inner("tls")?;
            assert_eq!(config, TlsConfig::from_paths("/ssl/cert.pem", "/ssl/key.pem"));

            jail.create_file("Rocket.toml", r#"
                [global.tls]
                certs = "cert.pem"
                key = "key.pem"
            "#)?;

            let config: TlsConfig = crate::Config::figment().extract_inner("tls")?;
            assert_eq!(config, TlsConfig::from_paths(
                jail.directory().join("cert.pem"),
                jail.directory().join("key.pem")
            ));

            jail.create_file("TLS.toml", r#"
                certs = "cert.pem"
                key = "key.pem"
                prefer_server_cipher_order = true
                ciphers = [
                    "TLS_CHACHA20_POLY1305_SHA256",
                    "TLS_AES_256_GCM_SHA384",
                    "TLS_AES_128_GCM_SHA256",
                    "TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256",
                    "TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384",
                    "TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256",
                ]
            "#)?;

            let config: TlsConfig = Figment::from(Toml::file("TLS.toml")).extract()?;
            let cert_path = jail.directory().join("cert.pem");
            let key_path = jail.directory().join("key.pem");
            assert_eq!(config, TlsConfig::from_paths(cert_path, key_path)
                 .with_preferred_server_cipher_order(true)
                 .with_ciphers([
                     CipherSuite::TLS_CHACHA20_POLY1305_SHA256,
                     CipherSuite::TLS_AES_256_GCM_SHA384,
                     CipherSuite::TLS_AES_128_GCM_SHA256,
                     CipherSuite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256,
                     CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
                     CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256,
                 ]));

            jail.create_file("Rocket.toml", r#"
                [global]
                shutdown.ctrlc = 0
                ident = false

                [global.tls]
                certs = "/ssl/cert.pem"
                key = "/ssl/key.pem"

                [global.limits]
                forms = "1mib"
                json = "10mib"
                stream = "50kib"
            "#)?;

            let config: TlsConfig = crate::Config::figment().extract_inner("tls")?;
            assert_eq!(config, TlsConfig::from_paths("/ssl/cert.pem", "/ssl/key.pem"));

            jail.create_file("Rocket.toml", r#"
                [global.tls]
                certs = "cert.pem"
                key = "key.pem"
            "#)?;

            let config: TlsConfig = crate::Config::figment().extract_inner("tls")?;
            assert_eq!(config, TlsConfig::from_paths(
                jail.directory().join("cert.pem"),
                jail.directory().join("key.pem")
            ));

            jail.create_file("Rocket.toml", r#"
                [global.tls]
                certs = "cert.pem"
                key = "key.pem"
                prefer_server_cipher_order = true
                ciphers = [
                    "TLS_CHACHA20_POLY1305_SHA256",
                    "TLS_AES_256_GCM_SHA384",
                    "TLS_AES_128_GCM_SHA256",
                    "TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256",
                    "TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384",
                    "TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256",
                ]
            "#)?;

            let config: TlsConfig = crate::Config::figment().extract_inner("tls")?;
            let cert_path = jail.directory().join("cert.pem");
            let key_path = jail.directory().join("key.pem");
            assert_eq!(config, TlsConfig::from_paths(cert_path, key_path)
                 .with_preferred_server_cipher_order(true)
                 .with_ciphers([
                     CipherSuite::TLS_CHACHA20_POLY1305_SHA256,
                     CipherSuite::TLS_AES_256_GCM_SHA384,
                     CipherSuite::TLS_AES_128_GCM_SHA256,
                     CipherSuite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256,
                     CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
                     CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256,
                 ]));

            Ok(())
        });
    }

    macro_rules! tls_example_private_pem {
        ($k:expr) => {
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/tls/private/", $k)
        }
    }

    #[test]
    fn verify_load_private_keys_of_different_types() -> Result<()> {
        let key_paths = [
            tls_example_private_pem!("rsa_sha256_key.pem"),
            tls_example_private_pem!("ecdsa_nistp256_sha256_key_pkcs8.pem"),
            tls_example_private_pem!("ecdsa_nistp384_sha384_key_pkcs8.pem"),
            tls_example_private_pem!("ed25519_key.pem"),
        ];

        for key in key_paths {
            TlsConfig::from_paths("", key).load_key()?;
        }

        Ok(())
    }

    #[test]
    fn verify_load_certs_of_different_types() -> Result<()> {
        let cert_paths = [
            tls_example_private_pem!("rsa_sha256_cert.pem"),
            tls_example_private_pem!("ecdsa_nistp256_sha256_cert.pem"),
            tls_example_private_pem!("ecdsa_nistp384_sha384_cert.pem"),
            tls_example_private_pem!("ed25519_cert.pem"),
        ];

        for cert in cert_paths {
            TlsConfig::from_paths(cert, "").load_certs()?;
        }

        Ok(())
    }
}
