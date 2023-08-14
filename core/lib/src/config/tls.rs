use figment::value::magic::{Either, RelativePathBuf};
use serde::{Deserialize, Serialize};
use indexmap::IndexSet;

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
/// [`mtls`](crate::mtls) module. See [`MutualTls`] for configuration details.
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
/// use rocket::config::{Config, TlsConfig, CipherSuite};
///
/// #[launch]
/// fn rocket() -> _ {
///     let tls_config = TlsConfig::from_paths("/ssl/certs.pem", "/ssl/key.pem")
///         .with_ciphers(CipherSuite::TLS_V13_SET)
///         .with_preferred_server_cipher_order(true);
///
///     let config = Config {
///         tls: Some(tls_config),
///         ..Default::default()
///     };
///
///     rocket::custom(config)
/// }
/// ```
///
/// Or by creating a custom figment:
///
/// ```rust
/// use rocket::config::Config;
///
/// let figment = Config::figment()
///     .merge(("tls.certs", "path/to/certs.pem"))
///     .merge(("tls.key", vec![0; 32]));
/// #
/// # let config = rocket::Config::from(figment);
/// # let tls_config = config.tls.as_ref().unwrap();
/// # assert!(tls_config.certs().is_left());
/// # assert!(tls_config.key().is_right());
/// # assert_eq!(tls_config.ciphers().count(), 9);
/// # assert!(!tls_config.prefer_server_cipher_order());
/// ```
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(nightly, doc(cfg(feature = "tls")))]
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
    pub(crate) mutual: Option<MutualTls>,
}

/// Mutual TLS configuration.
///
/// Configuration works in concert with the [`mtls`](crate::mtls) module, which
/// provides a request guard to validate, verify, and retrieve client
/// certificates in routes.
///
/// By default, mutual TLS is disabled and client certificates are not required,
/// validated or verified. To enable mutual TLS, the `mtls` feature must be
/// enabled and support configured via two `tls.mutual` parameters:
///
///   * `ca_certs`
///
///     A required path to a PEM file or raw bytes to a DER-encoded X.509 TLS
///     certificate chain for the certificate authority to verify client
///     certificates against. When a path is configured in a file, such as
///     `Rocket.toml`, relative paths are interpreted as relative to the source
///     file's directory.
///
///   * `mandatory`
///
///     An optional boolean that control whether client authentication is
///     required.
///
///     When `true`, client authentication is required. TLS connections where
///     the client does not present a certificate are immediately terminated.
///     When `false`, the client is not required to present a certificate. In
///     either case, if a certificate _is_ presented, it must be valid or the
///     connection is terminated.
///
/// In a `Rocket.toml`, configuration might look like:
///
/// ```toml
/// [default.tls.mutual]
/// ca_certs = "/ssl/ca_cert.pem"
/// mandatory = true                # when absent, defaults to false
/// ```
///
/// Programmatically, configuration might look like:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// use rocket::config::{Config, TlsConfig, MutualTls};
///
/// #[launch]
/// fn rocket() -> _ {
///     let tls_config = TlsConfig::from_paths("/ssl/certs.pem", "/ssl/key.pem")
///         .with_mutual(MutualTls::from_path("/ssl/ca_cert.pem"));
///
///     let config = Config {
///         tls: Some(tls_config),
///         ..Default::default()
///     };
///
///     rocket::custom(config)
/// }
/// ```
///
/// Once mTLS is configured, the [`mtls::Certificate`](crate::mtls::Certificate)
/// request guard can be used to retrieve client certificates in routes.
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[cfg(feature = "mtls")]
#[cfg_attr(nightly, doc(cfg(feature = "mtls")))]
pub struct MutualTls {
    /// Path to a PEM file with, or raw bytes for, DER-encoded Certificate
    /// Authority certificates which will be used to verify client-presented
    /// certificates.
    // TODO: We should support more than one root.
    pub(crate) ca_certs: Either<RelativePathBuf, Vec<u8>>,
    /// Whether the client is required to present a certificate.
    ///
    /// When `true`, the client is required to present a valid certificate to
    /// proceed with TLS. When `false`, the client is not required to present a
    /// certificate. In either case, if a certificate _is_ presented, it must be
    /// valid or the connection is terminated.
    #[serde(default)]
    #[serde(deserialize_with = "figment::util::bool_from_str_or_int")]
    pub mandatory: bool,
}

/// A supported TLS cipher suite.
#[allow(non_camel_case_types)]
#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash, Deserialize, Serialize)]
#[cfg_attr(nightly, doc(cfg(feature = "tls")))]
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

impl TlsConfig {
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

    /// Constructs a `TlsConfig` from paths to a `certs` certificate chain
    /// a `key` private-key. This method does no validation; it simply creates a
    /// structure suitable for passing into a [`Config`](crate::Config).
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::TlsConfig;
    ///
    /// let tls_config = TlsConfig::from_paths("/ssl/certs.pem", "/ssl/key.pem");
    /// ```
    pub fn from_paths<C, K>(certs: C, key: K) -> Self
        where C: AsRef<std::path::Path>, K: AsRef<std::path::Path>
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
    /// use rocket::config::TlsConfig;
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
    /// use rocket::config::{TlsConfig, CipherSuite};
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
    /// use rocket::config::{TlsConfig, CipherSuite};
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
    /// use rocket::config::{TlsConfig, CipherSuite};
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
    pub fn with_ciphers<I>(mut self, ciphers: I) -> Self
        where I: IntoIterator<Item = CipherSuite>
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
    /// use rocket::config::{TlsConfig, CipherSuite};
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

    /// Configures mutual TLS. See [`MutualTls`] for details.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::{TlsConfig, MutualTls};
    ///
    /// # let certs = &[];
    /// # let key = &[];
    /// let mtls_config = MutualTls::from_path("path/to/cert.pem").mandatory(true);
    /// let tls_config = TlsConfig::from_bytes(certs, key).with_mutual(mtls_config);
    /// assert!(tls_config.mutual().is_some());
    /// ```
    #[cfg(feature = "mtls")]
    #[cfg_attr(nightly, doc(cfg(feature = "mtls")))]
    pub fn with_mutual(mut self, config: MutualTls) -> Self {
        self.mutual = Some(config);
        self
    }

    /// Returns the value of the `certs` parameter.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Config;
    ///
    /// let figment = Config::figment()
    ///     .merge(("tls.certs", vec![0; 32]))
    ///     .merge(("tls.key", "/etc/ssl/key.pem"));
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
    ///
    /// let figment = Config::figment()
    ///     .merge(("tls.certs", vec![0; 32]))
    ///     .merge(("tls.key", "/etc/ssl/key.pem"));
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

    /// Returns an iterator over the enabled cipher suites in their order of
    /// preference from most to least preferred.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::{TlsConfig, CipherSuite};
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
    /// use rocket::config::TlsConfig;
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
    /// use rocket::config::{TlsConfig, MutualTls};
    ///
    /// # let certs = &[];
    /// # let key = &[];
    /// let mtls_config = MutualTls::from_path("path/to/cert.pem").mandatory(true);
    /// let tls_config = TlsConfig::from_bytes(certs, key).with_mutual(mtls_config);
    ///
    /// let mtls = tls_config.mutual().unwrap();
    /// assert_eq!(mtls.ca_certs().unwrap_left(), Path::new("path/to/cert.pem"));
    /// assert!(mtls.mandatory);
    /// ```
    #[cfg(feature = "mtls")]
    #[cfg_attr(nightly, doc(cfg(feature = "mtls")))]
    pub fn mutual(&self) -> Option<&MutualTls> {
        self.mutual.as_ref()
    }
}

#[cfg(feature = "mtls")]
impl MutualTls {
    /// Constructs a `MutualTls` from a path to a PEM file with a certificate
    /// authority `ca_certs` DER-encoded X.509 TLS certificate chain. This
    /// method does no validation; it simply creates a structure suitable for
    /// passing into a [`TlsConfig`].
    ///
    /// These certificates will be used to verify client-presented certificates
    /// in TLS connections.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::MutualTls;
    ///
    /// let tls_config = MutualTls::from_path("/ssl/ca_certs.pem");
    /// ```
    pub fn from_path<C: AsRef<std::path::Path>>(ca_certs: C) -> Self {
        MutualTls {
            ca_certs: Either::Left(ca_certs.as_ref().to_path_buf().into()),
            mandatory: Default::default()
        }
    }

    /// Constructs a `MutualTls` from a byte buffer to a certificate authority
    /// `ca_certs` DER-encoded X.509 TLS certificate chain. This method does no
    /// validation; it simply creates a structure suitable for passing into a
    /// [`TlsConfig`].
    ///
    /// These certificates will be used to verify client-presented certificates
    /// in TLS connections.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::MutualTls;
    ///
    /// # let ca_certs_buf = &[];
    /// let mtls_config = MutualTls::from_bytes(ca_certs_buf);
    /// ```
    pub fn from_bytes(ca_certs: &[u8]) -> Self {
        MutualTls {
            ca_certs: Either::Right(ca_certs.to_vec()),
            mandatory: Default::default()
        }
    }

    /// Sets whether client authentication is required. Disabled by default.
    ///
    /// When `true`, client authentication will be required. TLS connections
    /// where the client does not present a certificate will be immediately
    /// terminated. When `false`, the client is not required to present a
    /// certificate. In either case, if a certificate _is_ presented, it must be
    /// valid or the connection is terminated.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::MutualTls;
    ///
    /// # let ca_certs_buf = &[];
    /// let mtls_config = MutualTls::from_bytes(ca_certs_buf).mandatory(true);
    /// ```
    pub fn mandatory(mut self, mandatory: bool) -> Self {
        self.mandatory = mandatory;
        self
    }

    /// Returns the value of the `ca_certs` parameter.
    /// # Example
    ///
    /// ```rust
    /// use rocket::config::MutualTls;
    ///
    /// # let ca_certs_buf = &[];
    /// let mtls_config = MutualTls::from_bytes(ca_certs_buf).mandatory(true);
    /// assert_eq!(mtls_config.ca_certs().unwrap_right(), ca_certs_buf);
    /// ```
    pub fn ca_certs(&self) -> either::Either<std::path::PathBuf, &[u8]> {
        match &self.ca_certs {
            Either::Left(path) => either::Either::Left(path.relative()),
            Either::Right(bytes) => either::Either::Right(&bytes),
        }
    }
}

#[cfg(feature = "tls")]
mod with_tls_feature {
    use std::fs;
    use std::io::{self, Error};

    use crate::http::tls::Config;
    use crate::http::tls::rustls::SupportedCipherSuite as RustlsCipher;
    use crate::http::tls::rustls::cipher_suite;

    use yansi::Paint;

    use super::{Either, RelativePathBuf, TlsConfig, CipherSuite};

    type Reader = Box<dyn std::io::BufRead + Sync + Send>;

    fn to_reader(value: &Either<RelativePathBuf, Vec<u8>>) -> io::Result<Reader> {
        match value {
            Either::Left(path) => {
                let path = path.relative();
                let file = fs::File::open(&path).map_err(move |e| {
                    let source = figment::Source::File(path);
                    let msg = format!("error reading TLS file `{}`: {}", source.primary(), e);
                    Error::new(e.kind(), msg)
                })?;

                Ok(Box::new(io::BufReader::new(file)))
            }
            Either::Right(vec) => Ok(Box::new(io::Cursor::new(vec.clone()))),
        }
    }

    impl TlsConfig {
        /// This is only called when TLS is enabled.
        pub(crate) fn to_native_config(&self) -> io::Result<Config<Reader>> {
            Ok(Config {
                cert_chain: to_reader(&self.certs)?,
                private_key: to_reader(&self.key)?,
                ciphersuites: self.rustls_ciphers().collect(),
                prefer_server_order: self.prefer_server_cipher_order,
                #[cfg(not(feature = "mtls"))]
                mandatory_mtls: false,
                #[cfg(not(feature = "mtls"))]
                ca_certs: None,
                #[cfg(feature = "mtls")]
                mandatory_mtls: self.mutual.as_ref().map_or(false, |m| m.mandatory),
                #[cfg(feature = "mtls")]
                ca_certs: match self.mutual {
                    Some(ref mtls) => Some(to_reader(&mtls.ca_certs)?),
                    None => None
                },
            })
        }

        fn rustls_ciphers(&self) -> impl Iterator<Item = RustlsCipher> + '_ {
            self.ciphers().map(|ciphersuite| match ciphersuite {
                CipherSuite::TLS_CHACHA20_POLY1305_SHA256 =>
                    cipher_suite::TLS13_CHACHA20_POLY1305_SHA256,
                CipherSuite::TLS_AES_256_GCM_SHA384 =>
                    cipher_suite::TLS13_AES_256_GCM_SHA384,
                CipherSuite::TLS_AES_128_GCM_SHA256 =>
                    cipher_suite::TLS13_AES_128_GCM_SHA256,
                CipherSuite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256 =>
                    cipher_suite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256,
                CipherSuite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256 =>
                    cipher_suite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
                CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384 =>
                    cipher_suite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
                CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256 =>
                    cipher_suite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256,
                CipherSuite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384 =>
                    cipher_suite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
                CipherSuite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256 =>
                    cipher_suite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
            })
        }
    }
}
