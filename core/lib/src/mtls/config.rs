use std::io;

use figment::value::magic::{RelativePathBuf, Either};
use serde::{Serialize, Deserialize};

use crate::tls::{Result, Error};

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
/// use rocket::mtls::MtlsConfig;
/// use rocket::figment::providers::Serialized;
///
/// #[launch]
/// fn rocket() -> _ {
///     let mtls = MtlsConfig::from_path("/ssl/ca_cert.pem");
///     rocket::custom(rocket::Config::figment().merge(("tls.mutual", mtls)))
/// }
/// ```
///
/// Once mTLS is configured, the [`mtls::Certificate`](crate::mtls::Certificate)
/// request guard can be used to retrieve client certificates in routes.
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
pub struct MtlsConfig {
    /// Path to a PEM file with, or raw bytes for, DER-encoded Certificate
    /// Authority certificates which will be used to verify client-presented
    /// certificates.
    // TODO: Support more than one CA root.
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

impl MtlsConfig {
    /// Constructs a `MtlsConfig` from a path to a PEM file with a certificate
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
    /// use rocket::mtls::MtlsConfig;
    ///
    /// let tls_config = MtlsConfig::from_path("/ssl/ca_certs.pem");
    /// ```
    pub fn from_path<C: AsRef<std::path::Path>>(ca_certs: C) -> Self {
        MtlsConfig {
            ca_certs: Either::Left(ca_certs.as_ref().to_path_buf().into()),
            mandatory: Default::default()
        }
    }

    /// Constructs a `MtlsConfig` from a byte buffer to a certificate authority
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
    /// use rocket::mtls::MtlsConfig;
    ///
    /// # let ca_certs_buf = &[];
    /// let mtls_config = MtlsConfig::from_bytes(ca_certs_buf);
    /// ```
    pub fn from_bytes(ca_certs: &[u8]) -> Self {
        MtlsConfig {
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
    /// use rocket::mtls::MtlsConfig;
    ///
    /// # let ca_certs_buf = &[];
    /// let mtls_config = MtlsConfig::from_bytes(ca_certs_buf).mandatory(true);
    /// ```
    pub fn mandatory(mut self, mandatory: bool) -> Self {
        self.mandatory = mandatory;
        self
    }

    /// Returns the value of the `ca_certs` parameter.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::mtls::MtlsConfig;
    ///
    /// # let ca_certs_buf = &[];
    /// let mtls_config = MtlsConfig::from_bytes(ca_certs_buf).mandatory(true);
    /// assert_eq!(mtls_config.ca_certs().unwrap_right(), ca_certs_buf);
    /// ```
    pub fn ca_certs(&self) -> either::Either<std::path::PathBuf, &[u8]> {
        match &self.ca_certs {
            Either::Left(path) => either::Either::Left(path.relative()),
            Either::Right(bytes) => either::Either::Right(bytes),
        }
    }

    #[inline(always)]
    pub fn ca_certs_reader(&self) -> io::Result<Box<dyn io::BufRead + Sync + Send>> {
        crate::tls::config::to_reader(&self.ca_certs)
    }

    /// Load and decode CA certificates from `reader`.
    pub(crate) fn load_ca_certs(&self) -> Result<rustls::RootCertStore> {
        let mut roots = rustls::RootCertStore::empty();
        for cert in rustls_pemfile::certs(&mut self.ca_certs_reader()?) {
            roots.add(cert?).map_err(Error::CertAuth)?;
        }

        Ok(roots)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use figment::{Figment, providers::{Toml, Format}};

    use crate::mtls::MtlsConfig;

    #[test]
    fn test_mtls_config() {
        figment::Jail::expect_with(|jail| {
            jail.create_file("MTLS.toml", r#"
                certs = "/ssl/cert.pem"
                key = "/ssl/key.pem"
            "#)?;

            let figment = || Figment::from(Toml::file("MTLS.toml"));
            figment().extract::<MtlsConfig>().expect_err("no ca");

            jail.create_file("MTLS.toml", r#"
                ca_certs = "/ssl/ca.pem"
            "#)?;

            let mtls: MtlsConfig = figment().extract()?;
            assert_eq!(mtls.ca_certs().unwrap_left(), Path::new("/ssl/ca.pem"));
            assert!(!mtls.mandatory);

            jail.create_file("MTLS.toml", r#"
                ca_certs = "/ssl/ca.pem"
                mandatory = true
            "#)?;

            let mtls: MtlsConfig = figment().extract()?;
            assert_eq!(mtls.ca_certs().unwrap_left(), Path::new("/ssl/ca.pem"));
            assert!(mtls.mandatory);

            jail.create_file("MTLS.toml", r#"
                ca_certs = "relative/ca.pem"
            "#)?;

            let mtls: MtlsConfig = figment().extract()?;
            assert_eq!(mtls.ca_certs().unwrap_left(), jail.directory().join("relative/ca.pem"));

            Ok(())
        });
    }
}
