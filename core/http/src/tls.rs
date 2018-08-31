extern crate rustls;
extern crate hyper_sync_rustls;
extern crate dns_lookup;
extern crate untrusted;
extern crate webpki;
extern crate openssl;

pub use self::hyper_sync_rustls::{util, WrappedStream, ServerSession, TlsServer};
pub use self::rustls::{Certificate, PrivateKey, RootCertStore, internal::pemfile};
pub use self::dns_lookup::lookup_addr;
pub use self::untrusted::Input;
pub use self::webpki::{EndEntityCert, DNSNameRef};

use self::openssl::x509::X509;

/// Client MTLS certificate information.
///
/// The `MutualTlsUser` type specifies MTLS being required for the route and retrieves client
/// certificate information.
///
/// #Usage
///
/// A `MutualTlsUser` can be retrieved via its `FromRequest` implementation as a request guard.
/// Information of the certificate with a matching common name as a reverse DNS lookup of the
/// client IP address from the accepted certificate chain can be retrieved via the
/// `get_common_names`, `get_not_before`, and `get_not_after`.
///
/// ##Examples
///
/// The following short snippet shows `MutualTlsUser` being used as a request guard in a handler to
/// verify the client's certificate and print the common names of the client.
///
/// ```rust
/// # #![feature(plugin, decl_macro)]
/// # #![plugin(rocket_codegen)]
/// # extern crate rocket;
/// use rocket::http::tls::MutualTlsUser;
///
/// #[get("/message")]
/// fn message(mtls:MutualTlsUser) {
///     let common_names = mtls.get_common_names();
///     for name in common_names {
///         println!("{}", name);
///     }
/// }
///
/// # fn main() { }
/// ```
///
#[derive(Debug)]
pub struct MutualTlsUser {
    common_names: Vec<String>,
    not_before: String,
    not_after: String,
}

impl MutualTlsUser {
    // TODO: return a Result
    pub fn new(peer_cert: &Certificate) -> Option<MutualTlsUser> {
        // Generate an x509 using the certificate provided
        let x509 = X509::from_der(peer_cert.as_ref()).ok()?;

        // Retrieve alt names and store them into a Vec<String>
        let alt_names = x509.subject_alt_names()?;
        let mut common_names = Vec::new();
        for name in alt_names {
            common_names.push(name.dnsname()?.to_string())
        }

        // Retrieve certificate start time
        let not_before = format!("{}", x509.not_before());

        // Retrieve certificate end time
        let not_after = format!("{}", x509.not_after());

        Some(MutualTlsUser {
            common_names,
            not_before,
            not_after,
        })
    }

    /// Return the client's common names.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::tls::MutualTlsUser;
    ///
    /// fn handler(mtls: MutualTlsUser) {
    ///     let cert_common_names = mtls.get_common_names();
    /// }
    /// ```
    pub fn get_common_names(&self) -> &[String] {
        &self.common_names
    }

    /// Return the client's certificate's validity period start time.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::tls::MutualTlsUser;
    ///
    /// fn handler(mtls: MutualTlsUser) {
    ///     let cert_start_time = mtls.get_not_before();
    /// }
    /// ```
    pub fn get_not_before(&self) -> &str {
        self.not_before.as_str()
    }

    /// Return the client's certificate's validity period end time.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::tls::MutualTlsUser;
    ///
    /// fn handler(mtls: MutualTlsUser) {
    ///     let cert_end_time = mtls.get_not_after();
    /// }
    /// ```
    pub fn get_not_after(&self) -> &str {
        self.not_after.as_str()
    }
}
