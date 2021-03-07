use rocket::config::{Config, TlsConfig, V12Ciphers, V13Ciphers};

macro_rules! crate_relative {
    ($path:expr) => {
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../", $path))
    };
}

#[rocket::async_test]
async fn tls_launch_successfully() {
    let cert_path = crate_relative!("examples/tls/private/rsa_sha256_cert.pem");
    let key_path = crate_relative!("examples/tls/private/rsa_sha256_key.pem");

    let v12_ciphers = vec![V12Ciphers::EcdheRsaWithAes256GcmSha384];
    let v13_ciphers = vec![V13Ciphers::Aes128GcmSha256];

    let tls = Some(TlsConfig::from_paths(cert_path, key_path, true, v12_ciphers, v13_ciphers));

    let rocket = rocket::custom(Config { tls, ..Config::debug_default() });

    let shutdown_handle = rocket.shutdown();

    let join_handle = rocket::tokio::spawn(rocket.launch());

    shutdown_handle.shutdown();

    join_handle.await.unwrap().unwrap();
}
