use std::io::{self, Cursor, Read};

use rustls::{internal::pemfile, Certificate, PrivateKey, RootCertStore};

fn err(message: impl Into<std::borrow::Cow<'static, str>>) -> io::Error {
    io::Error::new(io::ErrorKind::Other, message.into())
}

/// Loads certificates from `reader`.
pub fn load_certs(reader: &mut dyn io::BufRead) -> io::Result<Vec<Certificate>> {
    pemfile::certs(reader).map_err(|_| err("invalid certificate"))
}

/// Load and decode the private key  from `reader`.
pub fn load_private_key(reader: &mut dyn io::BufRead) -> io::Result<PrivateKey> {
    // "rsa" (PKCS1) PEM files have a different first-line header than PKCS8
    // PEM files, use that to determine the parse function to use.
    let mut first_line = String::new();
    reader.read_line(&mut first_line)?;

    let private_keys_fn = match first_line.trim_end() {
        "-----BEGIN RSA PRIVATE KEY-----" => pemfile::rsa_private_keys,
        "-----BEGIN PRIVATE KEY-----" => pemfile::pkcs8_private_keys,
        _ => return Err(err("invalid key header"))
    };

    let key = private_keys_fn(&mut Cursor::new(first_line).chain(reader))
        .map_err(|_| err("invalid key file"))
        .and_then(|mut keys| match keys.len() {
            0 => Err(err("no valid keys found; is the file malformed?")),
            1 => Ok(keys.remove(0)),
            n => Err(err(format!("expected 1 key, found {}", n))),
        })?;

    // Ensure we can use the key.
    rustls::sign::any_supported_type(&key)
        .map_err(|_| err("key parsed but is unusable"))
        .map(|_| key)
}

/// Load and decode CA certificates from `reader`.
pub fn load_ca_certs(reader: &mut dyn io::BufRead) -> io::Result<RootCertStore> {
    let mut roots = rustls::RootCertStore::empty();
    let (_, e) = roots.add_pem_file(reader).map_err(|_| err("PEM format error"))?;
    if e != 0 {
        return Err(err("validity checks failed"));
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

        load_private_key(&mut Cursor::new(rsa_sha256_key))?;
        load_private_key(&mut Cursor::new(ecdsa_nistp256_sha256_key))?;
        load_private_key(&mut Cursor::new(ecdsa_nistp384_sha384_key))?;
        load_private_key(&mut Cursor::new(ed2551_key))?;

        Ok(())
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
