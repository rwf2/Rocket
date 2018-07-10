extern crate rustls;
extern crate hyper_sync_rustls;
extern crate dns_lookup;
extern crate untrusted;
extern crate webpki;
extern crate openssl;

pub use self::hyper_sync_rustls::{util, WrappedStream, ServerSession, TlsServer};
pub use self::rustls::{Certificate, PrivateKey, RootCertStore};
pub use self::dns_lookup::lookup_addr;
pub use self::untrusted::Input;
pub use self::webpki::{EndEntityCert, DNSNameRef};

use self::openssl::x509::X509;

//#[derive(Debug)]
pub struct MutualTlsUser {
    common_names: Vec<String>,
    not_before: String,
    not_after: String,
    pem_public_key: Vec<u8>,
    der_public_key: Vec<u8>,
    signature: Vec<u8>,
}

impl MutualTlsUser {
    pub fn new(peer_cert: Certificate) -> MutualTlsUser {
        let x509 = X509::from_der(peer_cert.as_ref()).expect("certificate could not be parsed");

        let alt_names = x509.subject_alt_names().expect("subject alt names could not be retrieved");
        let mut common_names = Vec::new();
        for name in alt_names {
            let name = name.dnsname().expect("common name could not be converted to dns name").clone();
            common_names.push(format!("{}", name));
        }

        let not_before = x509.not_before().clone();
        let not_before = format!("{}", not_before);

        let not_after = x509.not_after().clone();
        let not_after = format!("{}", not_after);

        let pem_public_key = x509.public_key().unwrap().public_key_to_pem().unwrap();

        let der_public_key = x509.public_key().unwrap().public_key_to_der().unwrap();

        let signature = x509.signature().as_slice().to_vec();

        MutualTlsUser {
            common_names,
            not_before,
            not_after,
            pem_public_key,
            der_public_key,
            signature,
        }
    }

    /// Get the client's common names
    pub fn get_common_names(&self) -> Vec<String> {
        self.common_names.clone()
    }

    /// Get the client's validity period not before
    pub fn get_validity_period_not_before(&self) -> String {
        self.not_before.clone()
    }

    /// Get the client's validity period not after
    pub fn get_validity_period_not_after(&self) -> String {
        self.not_after.clone()
    }

    /// Get the client's public key in pem format
    pub fn get_pem_public_key(&self) -> Vec<u8> {
        self.pem_public_key.clone()
    }

    /// Get the client's public key in der format
    pub fn get_der_public_key(&self) -> Vec<u8> {
        self.der_public_key.clone()
    }

    /// Get the client's signature
    pub fn get_signature(&self) -> Vec<u8> {
        self.signature.clone()
    }
}
