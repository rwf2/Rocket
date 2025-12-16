#![cfg(feature = "tls")]

extern crate rocket_community as rocket;

use rocket::figment::providers::Serialized;
use rocket::fs::relative;
use rocket::local::asynchronous::Client;
use rocket::tls::{CipherSuite, TlsConfig};

#[rocket::async_test]
async fn can_launch_tls() {
    let cert_path = relative!("examples/tls/private/rsa_sha256_cert.pem");
    let key_path = relative!("examples/tls/private/rsa_sha256_key.pem");

    let tls = TlsConfig::from_paths(cert_path, key_path).with_ciphers([
        CipherSuite::TLS_AES_128_GCM_SHA256,
        CipherSuite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
    ]);

    let config = rocket::Config::figment().merge(Serialized::defaults(tls));
    let client = Client::debug(rocket::custom(config)).await.unwrap();
    client.rocket().shutdown().notify();
    client.rocket().shutdown().await;
}
