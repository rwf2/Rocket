macro_rules! crate_relative {
    ($path:expr) => {
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../", $path))
    };
}

#[test]
fn tls_config_from_source() {
    use rocket::config::{Config, TlsConfig, V12Ciphers, V13Ciphers};
    use rocket::figment::Figment;

    let cert_path = crate_relative!("examples/tls/private/rsa_sha256_cert.pem");
    let key_path = crate_relative!("examples/tls/private/rsa_sha256_key.pem");
    let prefer_server_ciphers_order = true;
    let v12_ciphers = vec![V12Ciphers::EcdheRsaWithAes256GcmSha384];
    let v13_ciphers = vec![V13Ciphers::Aes128GcmSha256];

    let rocket_config = Config {
        tls: Some(TlsConfig::from_paths(cert_path, key_path, true, v12_ciphers.clone(), v13_ciphers.clone())),
        ..Default::default()
    };

    let config: Config = Figment::from(rocket_config).extract().unwrap();
    let tls = config.tls.expect("have TLS config");
    assert_eq!(tls.certs().unwrap_left(), cert_path);
    assert_eq!(tls.key().unwrap_left(), key_path);
    assert_eq!(tls.prefer_server_ciphers_order, prefer_server_ciphers_order);
    assert_eq!(tls.v12_ciphers, v12_ciphers);
    assert_eq!(tls.v13_ciphers, v13_ciphers);
}
