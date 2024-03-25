use std::io;

use rustls::RootCertStore;
use rustls::crypto::CryptoProvider;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};

use crate::tls::error::{Result, Error, KeyError};

/// Loads certificates from `reader`.
pub fn load_cert_chain(reader: &mut dyn io::BufRead) -> Result<Vec<CertificateDer<'static>>> {
    rustls_pemfile::certs(reader)
        .collect::<Result<_, _>>()
        .map_err(Error::CertChain)
}

/// Load and decode the private key  from `reader`.
pub fn load_key(reader: &mut dyn io::BufRead) -> Result<PrivateKeyDer<'static>> {
    use rustls_pemfile::Item::*;

    let mut keys: Vec<PrivateKeyDer<'static>> = rustls_pemfile::read_all(reader)
        .map(|result| result.map_err(KeyError::Io)
            .and_then(|item| match item {
                Pkcs1Key(key) => Ok(key.into()),
                Pkcs8Key(key) => Ok(key.into()),
                Sec1Key(key) => Ok(key.into()),
                _ => Err(KeyError::BadItem(item))
            })
        )
        .collect::<Result<_, _>>()?;

    if keys.len() != 1 {
        return Err(KeyError::BadKeyCount(keys.len()).into());
    }

    // Ensure we can use the key.
    let key = keys.remove(0);
    get_crypto_provider().key_provider.load_private_key(key.clone_key())
        .map_err(KeyError::Unsupported)?;
    Ok(key)
}

/// Load and decode CA certificates from `reader`.
pub fn load_ca_certs(reader: &mut dyn io::BufRead) -> Result<RootCertStore> {
    let mut roots = rustls::RootCertStore::empty();
    for cert in load_cert_chain(reader)? {
        roots.add(cert).map_err(Error::CertAuth)?;
    }

    Ok(roots)
}

pub(crate) fn get_crypto_provider() -> CryptoProvider {
    if let Some(crypto_provider) = rustls::crypto::CryptoProvider::get_default() {
        CryptoProvider::clone(crypto_provider)
    } else {
        let crypto_provider = rustls::crypto::ring::default_provider();
        // Should only fail due to other concurrent install, so we ignore it
        let _ = crypto_provider.clone().install_default();

        crypto_provider
    }

}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! tls_example_key {
        ($k:expr) => {
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/tls/private/", $k))
        }
    }

    #[test]
    fn verify_load_private_keys_of_different_types() -> Result<()> {
        let rsa_sha256_key = tls_example_key!("rsa_sha256_key.pem");
        let ecdsa_nistp256_sha256_key = tls_example_key!("ecdsa_nistp256_sha256_key_pkcs8.pem");
        let ecdsa_nistp384_sha384_key = tls_example_key!("ecdsa_nistp384_sha384_key_pkcs8.pem");
        let ed2551_key = tls_example_key!("ed25519_key.pem");

        load_key(&mut &rsa_sha256_key[..])?;
        load_key(&mut &ecdsa_nistp256_sha256_key[..])?;
        load_key(&mut &ecdsa_nistp384_sha384_key[..])?;
        load_key(&mut &ed2551_key[..])?;

        Ok(())
    }

    #[test]
    fn verify_load_certs_of_different_types() -> Result<()> {
        let rsa_sha256_cert = tls_example_key!("rsa_sha256_cert.pem");
        let ecdsa_nistp256_sha256_cert = tls_example_key!("ecdsa_nistp256_sha256_cert.pem");
        let ecdsa_nistp384_sha384_cert = tls_example_key!("ecdsa_nistp384_sha384_cert.pem");
        let ed2551_cert = tls_example_key!("ed25519_cert.pem");

        load_cert_chain(&mut &rsa_sha256_cert[..])?;
        load_cert_chain(&mut &ecdsa_nistp256_sha256_cert[..])?;
        load_cert_chain(&mut &ecdsa_nistp384_sha384_cert[..])?;
        load_cert_chain(&mut &ed2551_cert[..])?;

        Ok(())
    }
}
