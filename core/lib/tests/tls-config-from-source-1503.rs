#![cfg(feature = "tls")]

macro_rules! relative {
    ($path:expr) => {
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/", $path))
    };
}

#[test]
fn tls_config_from_source() {
    use rocket::config::{Config, TlsConfig};
    use rocket::figment::Figment;

    let cert_path = relative!("examples/tls/private/cert.pem");
    let key_path = relative!("examples/tls/private/key.pem");

    let rocket_config = Config {
        tls: Some(TlsConfig::from_paths(cert_path, key_path)),
        ..Default::default()
    };

    let config: Config = Figment::from(rocket_config).extract().unwrap();
    let tls = config.tls.expect("have TLS config");
    assert_eq!(tls.certs().unwrap_left(), cert_path);
    assert_eq!(tls.key().unwrap_left(), key_path);
}
