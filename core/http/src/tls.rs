pub use self::hyper_sync_rustls::{util, WrappedStream, ServerSession, TlsServer};
pub use self::rustls::{Certificate, PrivateKey, RootCertStore, internal::pemfile};
pub use self::dns_lookup::lookup_addr;

use self::untrusted::Input;
use self::webpki::{EndEntityCert, DNSNameRef};


/// Find the first `Certificate` valid for the given DNS name
fn first_valid_cert_for_name<'a>(dns_name: DNSNameRef, certs: &'a [Certificate]) -> Option<&'a Certificate> {
    certs.iter()
        .find(|cert| {
            let cert_input = Input::from(cert.as_ref());
            EndEntityCert::from(cert_input)
                .and_then(|ee| ee.verify_is_valid_for_dns_name(dns_name).map(|_| true))
                .unwrap_or(false)
        })
}

/// Given a domain name and a set of `Certificate`s, return the first certificate
/// that matches the domain name
pub fn find_valid_cert_for_peer<'a>(name: &'a str, certs: &'a [Certificate]) -> Result<&'a Certificate, ()> {
    let input = Input::from(name.as_bytes());
    let domain_name = DNSNameRef::try_from_ascii(input)?;

    // Find the first valid cert for the given name
    let valid_cert = first_valid_cert_for_name(domain_name, &certs).ok_or(())?;

    Ok(valid_cert)
}

/// MTLS client authentication.
///
/// The `MutualTlsUser` type is a request guard that only allows properly authenticated clients.
///
/// #Usage
///
/// A `MutualTlsUser` can be retrieved via its `FromRequest` implementation as a request guard.
///
/// ##Examples
///
/// The following short snippet shows `MutualTlsUser` being used as a request guard in a handler to
/// verify the client's certificate and print its subject name.
///
/// ```rust
/// # #![feature(plugin, decl_macro)]
/// # #![plugin(rocket_codegen)]
/// # extern crate rocket;
/// use rocket::http::tls::MutualTlsUser;
///
/// #[get("/message")]
/// fn message(mtls: MutualTlsUser) {
///     println!("{}", mtls.subject_name());
/// }
///
/// # fn main() { }
/// ```
///
#[derive(Debug)]
pub struct MutualTlsUser {
    subject_name: String,
}

impl MutualTlsUser {
    pub fn new(subject_name: &str) -> MutualTlsUser {
        // NOTE: `subject_name` is not necessarily the subject name in the certificate,
        // but it is the name for which the certificate was validated.
        MutualTlsUser {
            subject_name: subject_name.to_string()
        }
    }

    /// Return the client's subject name.
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate rocket;
    /// use rocket::http::tls::MutualTlsUser;
    ///
    /// fn handler(mtls: MutualTlsUser) {
    ///     let subject_name = mtls.subject_name();
    /// }
    /// ```
    pub fn subject_name(&self) -> &str {
        &self.subject_name
    }
}
