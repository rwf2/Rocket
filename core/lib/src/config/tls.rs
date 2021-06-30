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
///     must be a DER-encoded ASN.1 key in either PKCS#8 or PKCS#1 format.
///     When a path is configured in a file, such as `Rocket.toml`, relative
///     paths are interpreted as relative to the source file's directory.
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
/// The following example illustrates manual configuration:
///
/// ```rust
/// use rocket::config::{Config, TlsConfig, CipherSuite};
///
/// // From a manually constructed figment.
/// let figment = rocket::Config::figment()
///     .merge(("tls.certs", "path/to/certs.pem"))
///     .merge(("tls.key", vec![0; 32]));
///
/// let config = rocket::Config::from(figment);
/// let tls_config = config.tls.as_ref().unwrap();
/// assert!(tls_config.certs().is_left());
/// assert!(tls_config.key().is_right());
/// assert_eq!(tls_config.ciphers().count(), 9);
/// assert!(!tls_config.prefer_server_cipher_order());
///
/// // From a serialized `TlsConfig`.
/// let tls_config = TlsConfig::from_paths("/ssl/certs.pem", "/ssl/key.pem")
///     .with_ciphers(CipherSuite::TLS_V13_SET)
///     .with_preferred_server_cipher_order(true);
///
/// let figment = rocket::Config::figment()
///     .merge(("tls", tls_config));
///
/// let config = rocket::Config::from(figment);
/// let tls_config = config.tls.as_ref().unwrap();
/// assert_eq!(tls_config.ciphers().count(), 3);
/// assert!(tls_config.prefer_server_cipher_order());
/// ```
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TlsConfig {
    /// Path or raw bytes for the DER-encoded X.509 TLS certificate chain.
    pub(crate) certs: Either<RelativePathBuf, Vec<u8>>,
    /// Path or raw bytes to DER-encoded ASN.1 key in either PKCS#8 or PKCS#1
    /// format.
    pub(crate) key: Either<RelativePathBuf, Vec<u8>>,
    /// List of TLS cipher suites in server-preferred order.
    #[serde(default = "CipherSuite::default_set")]
    pub(crate) ciphers: IndexSet<CipherSuite>,
    /// Whether to prefer the server's cipher suite order over the client's.
    #[serde(default)]
    pub(crate) prefer_server_cipher_order: bool,
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
    /// Constructs a `TlsConfig` from paths to a `certs` certificate-chain
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
            ciphers: CipherSuite::default_set(),
            prefer_server_cipher_order: Default::default(),
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
    ///
    /// # let certs_buf = &[];
    /// # let key_buf = &[];
    /// let tls_config = TlsConfig::from_bytes(certs_buf, key_buf);
    /// ```
    pub fn from_bytes(certs: &[u8], key: &[u8]) -> Self {
        TlsConfig {
            certs: Either::Right(certs.to_vec()),
            key: Either::Right(key.to_vec()),
            ciphers: CipherSuite::default_set(),
            prefer_server_cipher_order: Default::default(),
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
    /// // Which can be overriden with the eponymous builder method.
    /// let tls_config = TlsConfig::from_bytes(certs_buf, key_buf)
    ///     .with_preferred_server_cipher_order(true);
    ///
    /// assert!(tls_config.prefer_server_cipher_order());
    /// ```
    pub fn prefer_server_cipher_order(&self) -> bool {
        self.prefer_server_cipher_order
    }
}

#[cfg(feature = "tls")]
mod with_tls_feature {
    use crate::http::private::tls::rustls::SupportedCipherSuite as RustlsCipher;
    use crate::http::private::tls::rustls::ciphersuite as rustls;

    use super::*;

    type Reader = Box<dyn std::io::BufRead + Sync + Send>;

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

        pub(crate) fn rustls_ciphers(&self) -> impl Iterator<Item = &'static RustlsCipher> + '_ {
            self.ciphers().map(|ciphersuite| match ciphersuite {
                CipherSuite::TLS_CHACHA20_POLY1305_SHA256 =>
                    &rustls::TLS13_CHACHA20_POLY1305_SHA256,
                CipherSuite::TLS_AES_256_GCM_SHA384 =>
                    &rustls::TLS13_AES_256_GCM_SHA384,
                CipherSuite::TLS_AES_128_GCM_SHA256 =>
                    &rustls::TLS13_AES_128_GCM_SHA256,
                CipherSuite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256 =>
                    &rustls::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256,
                CipherSuite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256 =>
                    &rustls::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
                CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384 =>
                    &rustls::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
                CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256 =>
                    &rustls::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256,
                CipherSuite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384 =>
                    &rustls::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
                CipherSuite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256 =>
                    &rustls::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
            })
        }
    }
}
