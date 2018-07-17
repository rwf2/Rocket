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
/// `get_common_names`, `get_not_before`, `get_not_after`, `get_pem_public_key`,
/// `get_der_public_key`, and `get_signature` methods.
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
    pub fn new(peer_cert: Certificate) -> Option<MutualTlsUser> {
        // Generate an x509 using the certificate provided
        let x509 = X509::from_der(peer_cert.as_ref());
        if x509.is_err() {
            return None;
        }
        let x509 = x509.expect("Failed to generate X509.");

        // Retrieve alt names and store them into a Vec<String>
        let alt_names = x509.subject_alt_names();
        if alt_names.is_none() {
            return None;
        }
        let alt_names = alt_names.expect("Alt names for certificate do not exist.");
        let mut common_names = Vec::new();
        for name in alt_names {
            let name = name.dnsname();
            if name.is_none() {
                return None;
            }
            let name = name.expect("Name does not exist.");
            common_names.push(format!("{}", name));
        }

        // Retrieve certificate start time
        let not_before = x509.not_before().clone();
        let not_before = format!("{}", not_before);

        // Retrieve certificate end time
        let not_after = x509.not_after().clone();
        let not_after = format!("{}", not_after);

        Some(MutualTlsUser {
            common_names,
            not_before,
            not_after,
        })
    }

    /// Return a clone of the client's common names.
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
    pub fn get_common_names(&self) -> Vec<String> {
        self.common_names.clone()
    }

    /// Return a clone of the client's certificate's validity period start time.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::tls::MutualTlsUser;
    ///
    /// fn handler(mtls: MutualTlsUser) {
    ///     let cert_start_time = mtls.get_validity_period_not_before();
    /// }
    /// ```
    pub fn get_validity_period_not_before(&self) -> String {
        self.not_before.clone()
    }

    /// Return a clone of the client's certificate's validity period end time.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::tls::MutualTlsUser;
    ///
    /// fn handler(mtls: MutualTlsUser) {
    ///     let cert_end_time = mtls.get_validity_period_not_after();
    /// }
    /// ```
    pub fn get_validity_period_not_after(&self) -> String {
        self.not_after.clone()
    }
}
