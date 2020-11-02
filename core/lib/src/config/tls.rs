use figment::value::magic::{Either, RelativePathBuf};
use serde::{Deserialize, Serialize};

/// TLS configuration: a certificate chain and a private key.
///
/// Both `certs` and `key` can be configured as a path or as raw bytes. `certs`
/// must be a DER-encoded X.509 TLS certificate chain, while `key` must be a
/// DER-encoded ASN.1 key in either PKCS#8 or PKCS#1 format.
///
/// The following example illustrates manual configuration:
///
/// ```rust
/// # use rocket::figment::Figment;
/// let figment = Figment::from(rocket::Config::default())
///     .merge(("tls.certs", "strings/are/paths/certs.pem"))
///     .merge(("tls.key", vec![0; 32]));
///
/// let config = rocket::Config::from(figment);
/// let tls_config = config.tls.as_ref().unwrap();
/// assert!(tls_config.certs().is_left());
/// assert!(tls_config.key().is_right());
/// ```
///
/// When a path is configured in a file source, such as `Rocket.toml`, relative
/// paths are interpreted as being relative to the source file's directory.
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
pub struct TlsConfig {
    /// Path or raw bytes for the DER-encoded X.509 TLS certificate chain.
    pub(crate) certs: Either<RelativePathBuf, Vec<u8>>,
    /// Path or raw bytes to DER-encoded ASN.1 key in either PKCS#8 or PKCS#1
    /// format.
    pub(crate) key: Either<RelativePathBuf, Vec<u8>>,
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
            key: Either::Left(key.as_ref().to_path_buf().into())
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
            certs: Either::Right(certs.to_vec().into()),
            key: Either::Right(key.to_vec().into())
        }
    }

    /// Returns the value of the `certs` parameter.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::figment::Figment;
    /// let figment = Figment::from(rocket::Config::default())
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
    /// # use rocket::figment::Figment;
    /// # use std::path::Path;
    /// let figment = Figment::from(rocket::Config::default())
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
