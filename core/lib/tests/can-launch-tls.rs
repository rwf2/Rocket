#![cfg(feature = "tls")]

use rocket::fs::relative;
use rocket::config::{Config, TlsConfig, CipherSuite};
use rocket::local::asynchronous::Client;

#[rocket::async_test]
async fn can_launch_tls() {
    let cert_path = relative!("examples/tls/private/rsa_sha256_cert.pem");
    let key_path = relative!("examples/tls/private/rsa_sha256_key.pem");

    let tls = TlsConfig::from_paths(cert_path, key_path)
        .with_ciphers([
            CipherSuite::TLS_AES_128_GCM_SHA256,
            CipherSuite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
        ]);

    let rocket = rocket::custom(Config { tls: Some(tls), ..Config::debug_default() });
    let client = Client::debug(rocket).await.unwrap();

    client.rocket().shutdown().notify();
    client.rocket().shutdown().await;
}

#[rocket::async_test]
async fn invalid_tls_config_fails_at_bind() {
    use rocket::error::ErrorKind;

    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/tls/private/");
    let certs = std::fs::read(format!("{dir}rsa_sha256_cert.pem")).unwrap();
    let key = std::fs::read(format!("{dir}ed25519_key.pem")).unwrap();
    let tls = TlsConfig::from_bytes(&certs, &key);
    let config = Config { tls: Some(tls), port: 0, ..Config::debug_default() };
    let error = rocket::custom(config).launch().await.unwrap_err();
    assert!(matches!(error.kind(), ErrorKind::Bind(_)));

    #[cfg(feature = "mtls")]
    for mandatory in [false, true] {
        use rocket::config::MutualTls;

        let key = std::fs::read(format!("{dir}rsa_sha256_key.pem")).unwrap();
        let tls = TlsConfig::from_bytes(&certs, &key)
            .with_mutual(MutualTls::from_bytes(&[]).mandatory(mandatory));
        let config = Config { tls: Some(tls), port: 0, ..Config::debug_default() };
        let error = rocket::custom(config).launch().await.unwrap_err();
        assert!(matches!(error.kind(), ErrorKind::Bind(_)));
    }
}
