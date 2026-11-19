#![cfg(feature = "tls")]

use rocket::fs::relative;
use rocket::local::asynchronous::Client;
use rocket::tls::{TlsConfig, CipherSuite};
use rocket::figment::providers::Serialized;

#[rocket::async_test]
async fn can_launch_tls() {
    let cert_path = relative!("examples/tls/private/rsa_sha256_cert.pem");
    let key_path = relative!("examples/tls/private/rsa_sha256_key.pem");

    let tls = TlsConfig::from_paths(cert_path, key_path)
        .with_ciphers([
            CipherSuite::TLS_AES_128_GCM_SHA256,
            CipherSuite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
        ]);

    let config = rocket::Config::figment().merge(Serialized::defaults(tls));
    let client = Client::debug(rocket::custom(config)).await.unwrap();
    client.rocket().shutdown().notify();
    client.rocket().shutdown().await;
}

#[rocket::async_test]
async fn invalid_tls_config_fails_at_bind() {
    use rocket::error::ErrorKind;
    use rocket::figment::Figment;

    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/tls/private/");
    let certs = std::fs::read(format!("{dir}rsa_sha256_cert.pem")).unwrap();
    let key = std::fs::read(format!("{dir}ed25519_key.pem")).unwrap();
    let tls = TlsConfig::from_bytes(&certs, &key);
    let config = Figment::from(rocket::Config::debug_default())
        .merge(("port", 0))
        .merge(("tls", tls));

    let error = rocket::custom(config).launch().await.unwrap_err();
    assert!(matches!(error.kind(), ErrorKind::Bind(..)));

    #[cfg(feature = "mtls")]
    for mandatory in [false, true] {
        use rocket::mtls::MtlsConfig;

        let key = std::fs::read(format!("{dir}rsa_sha256_key.pem")).unwrap();
        let tls = TlsConfig::from_bytes(&certs, &key)
            .with_mutual(MtlsConfig::from_bytes(&[]).mandatory(mandatory));
        let config = Figment::from(rocket::Config::debug_default())
            .merge(("port", 0))
            .merge(("tls", tls));

        let error = rocket::custom(config).launch().await.unwrap_err();
        assert!(matches!(error.kind(), ErrorKind::Bind(..)));
    }
}
