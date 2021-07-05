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
