use std::io::{self, Cursor, Read};

use rustls::RootCertStore;
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use rustls_pki_types::pem::{PemObject, SectionKind};

fn err(message: impl Into<std::borrow::Cow<'static, str>>) -> io::Error {
    io::Error::new(io::ErrorKind::Other, message.into())
}

/// Loads certificates from `reader`.
pub fn load_certs(reader: &mut dyn io::BufRead) -> io::Result<Vec<CertificateDer<'static>>> {
    CertificateDer::pem_reader_iter(reader)
        .collect::<Result<_, _>>()
        .map_err(|_| err("invalid certificate"))
}

/// Load and decode the private key from `reader`.
pub fn load_private_key(reader: &mut dyn io::BufRead) -> io::Result<PrivateKeyDer<'static>> {
    // Select the first supported key format, ignoring other sections.
    let mut header = String::new();
    let kind = loop {
        header.clear();
        if reader.read_line(&mut header)? == 0 {
            return Err(err("failed to find key header; supported formats are: RSA, PKCS8, SEC1"));
        }

        break match header.trim_end() {
            "-----BEGIN RSA PRIVATE KEY-----" => SectionKind::RsaPrivateKey,
            "-----BEGIN PRIVATE KEY-----" => SectionKind::PrivateKey,
            "-----BEGIN EC PRIVATE KEY-----" => SectionKind::EcPrivateKey,
            _ => continue,
        };
    };

    let key = PrivateKeyDer::pem_reader_iter(Cursor::new(header).chain(reader))
        .filter(|key| match key {
            Ok(PrivateKeyDer::Pkcs1(_)) => kind == SectionKind::RsaPrivateKey,
            Ok(PrivateKeyDer::Pkcs8(_)) => kind == SectionKind::PrivateKey,
            Ok(PrivateKeyDer::Sec1(_)) => kind == SectionKind::EcPrivateKey,
            Ok(_) => false,
            Err(_) => true,
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| err("invalid key file"))
        .and_then(|mut keys| match keys.len() {
            0 => Err(err("no valid keys found; is the file malformed?")),
            1 => Ok(keys.remove(0)),
            n => Err(err(format!("expected 1 key, found {}", n))),
        })?;

    // Ensure we can use the key.
    rustls::crypto::ring::sign::any_supported_type(&key)
        .map_err(|_| err("key parsed but is unusable"))
        .map(|_| key)
}

/// Load and decode CA certificates from `reader`.
pub fn load_ca_certs(reader: &mut dyn io::BufRead) -> io::Result<RootCertStore> {
    let mut roots = rustls::RootCertStore::empty();
    for cert in load_certs(reader)? {
        roots.add(cert).map_err(|e| err(format!("CA cert error: {}", e)))?;
    }

    Ok(roots)
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
    fn verify_load_private_keys_of_different_types() -> io::Result<()> {
        let rsa_sha256_key = tls_example_key!("rsa_sha256_key.pem");
        let ecdsa_nistp256_sha256_key = tls_example_key!("ecdsa_nistp256_sha256_key_pkcs8.pem");
        let ecdsa_nistp384_sha384_key = tls_example_key!("ecdsa_nistp384_sha384_key_pkcs8.pem");
        let ed2551_key = tls_example_key!("ed25519_key.pem");
        let rsa_pkcs1_key = tls_example_key!("ca_key.pem");
        let ecdsa_sec1_key = tls_example_key!("ecdsa_nistp256_sha256_key_sec1.pem");

        load_private_key(&mut Cursor::new(rsa_sha256_key))?;
        load_private_key(&mut Cursor::new(ecdsa_nistp256_sha256_key))?;
        load_private_key(&mut Cursor::new(ecdsa_nistp384_sha384_key))?;
        load_private_key(&mut Cursor::new(ed2551_key))?;
        load_private_key(&mut Cursor::new(rsa_pkcs1_key))?;
        load_private_key(&mut Cursor::new(ecdsa_sec1_key))?;

        Ok(())
    }

    #[test]
    fn private_key_selection() {
        let keys: [&[u8]; 3] = [
            tls_example_key!("ca_key.pem"),
            tls_example_key!("ed25519_key.pem"),
            tls_example_key!("ecdsa_nistp256_sha256_key_sec1.pem"),
        ];

        let cert = tls_example_key!("ca_cert.pem");
        for (i, first) in keys.iter().enumerate() {
            let expected = load_private_key(&mut Cursor::new(first)).unwrap();
            let mut pem = cert.to_vec();
            pem.extend_from_slice(first);
            for other in keys.iter().cycle().skip(i + 1).take(2) {
                pem.extend_from_slice(other);
            }

            pem.extend_from_slice(cert);
            let actual = load_private_key(&mut Cursor::new(&pem)).unwrap();
            assert_eq!(actual.secret_der(), expected.secret_der());

            pem.extend_from_slice(first);
            let error = load_private_key(&mut Cursor::new(pem)).unwrap_err();
            assert_eq!(error.to_string(), "expected 1 key, found 2");
        }
    }

    #[test]
    fn private_key_errors() {
        let key = tls_example_key!("ed25519_key.pem");
        let malformed = b"-----BEGIN CERTIFICATE-----\n?\n-----END CERTIFICATE-----\n";
        let mut pem = malformed.to_vec();
        pem.extend_from_slice(key);
        assert!(load_private_key(&mut Cursor::new(&pem)).is_ok());

        pem.extend_from_slice(malformed);
        let error = load_private_key(&mut Cursor::new(pem)).unwrap_err();
        assert_eq!(error.to_string(), "invalid key file");

        let pem = b"-----BEGIN PRIVATE KEY-----\n?\n-----END PRIVATE KEY-----\n";
        let error = load_private_key(&mut Cursor::new(pem)).unwrap_err();
        assert_eq!(error.to_string(), "invalid key file");

        let error = load_private_key(&mut Cursor::new(malformed)).unwrap_err();
        assert!(error.to_string().starts_with("failed to find key header"));
    }

    #[test]
    fn verify_load_certs_of_different_types() -> io::Result<()> {
        let rsa_sha256_cert = tls_example_key!("rsa_sha256_cert.pem");
        let ecdsa_nistp256_sha256_cert = tls_example_key!("ecdsa_nistp256_sha256_cert.pem");
        let ecdsa_nistp384_sha384_cert = tls_example_key!("ecdsa_nistp384_sha384_cert.pem");
        let ed2551_cert = tls_example_key!("ed25519_cert.pem");

        load_certs(&mut Cursor::new(rsa_sha256_cert))?;
        load_certs(&mut Cursor::new(ecdsa_nistp256_sha256_cert))?;
        load_certs(&mut Cursor::new(ecdsa_nistp384_sha384_cert))?;
        load_certs(&mut Cursor::new(ed2551_cert))?;

        Ok(())
    }
}
