#![cfg(feature = "tls")]

#[macro_use] extern crate rocket;

use rocket_codegen::routes;

macro_rules! relative {
    ($path:expr) => {
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/", $path))
    };
}

#[get("/hello")]
fn tls_test_path()  -> &'static str{
    "world"
}

#[test]
fn tls_server_operation() {
    use std::io::Read;

    use rocket::config::{Config, TlsConfig, SecretKey};
    use rocket::figment::Figment;

    let secret_key = SecretKey::generate().expect("get key");
    let cert_path = relative!("../../examples/tls/private/rsa_sha256_cert.pem");
    let key_path = relative!("../../examples/tls/private/rsa_sha256_key.pem");
    let ca_cert_path = relative!("../../examples/tls/private/ca_cert.pem");

    let port = match std::net::TcpListener::bind(("127.0.0.1", 0)) {
            Ok(listener) => Some(listener.local_addr().unwrap().port()),
            Err(_) => None,
        };
    assert!(port.is_some());

    let rocket_config = Config {
        port: port.unwrap(),
        tls: Some(TlsConfig::from_paths(cert_path, key_path)),
        secret_key: secret_key,
        ..Default::default()
    };
    let config: Config = Figment::from(rocket_config).extract().unwrap();

    let (shutdown_signal_sender, mut shutdown_signal_receiver) = tokio::sync::mpsc::channel::<()>(1);

    // Create a runtime in a separate thread for the server being tested
    let join_handle = std::thread::spawn(move || {
        
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let task_handle = tokio::spawn( async {
                 rocket::custom(config).mount("/", routes![tls_test_path]).launch().await.unwrap();
            });
            shutdown_signal_receiver.recv().await;
            task_handle.abort();
        });

    });

    let request_url = format!("https://localhost:{}/hello", port.unwrap());

    // Certificates are not loaded, so request should fail
    assert!(reqwest::blocking::get(&request_url).is_err());

    // Load the CA certicate for use with test client
    let mut buf = Vec::new();
    std::fs::File::open(ca_cert_path).unwrap().read_to_end(&mut buf).unwrap();
    let cert = reqwest::Certificate::from_pem(&buf).unwrap();

    let client = reqwest::blocking::Client::builder().add_root_certificate(cert).build().unwrap();

    // Replace with multiple tests rather than fixed wait time
    std::thread::sleep(std::time::Duration::from_secs(5));

    let response = client.get(&request_url).send();
    assert_eq!(&response.unwrap().text().unwrap(), "world");

    shutdown_signal_sender.blocking_send(()).unwrap();
    join_handle.join().unwrap();

}




#[get("/hello")]
fn tls_test_path_state()  -> &'static str{
    "world"
}

#[test]
fn tls_server_operation_dynamic_certs() {
    use std::io::Read;

    use rocket::config::{Config, TlsConfig, SecretKey};
    use rocket::figment::Figment;

    let secret_key = SecretKey::generate().expect("get key");
    let cert_path = relative!("./tests/private/one/rsa_sha256_cert.pem");
    let key_path = relative!("./tests/private/one/rsa_sha256_key.pem");
    let ca_cert_path = relative!("./tests/private/one/ca_cert.pem");

    let port = match std::net::TcpListener::bind(("127.0.0.1", 0)) {
            Ok(listener) => Some(listener.local_addr().unwrap().port()),
            Err(_) => None,
        };
    assert!(port.is_some());

    let rocket_config = Config {
        port: port.unwrap(),
        tls: Some(TlsConfig::from_paths(cert_path, key_path)),
        secret_key: secret_key,
        ..Default::default()
    };
    let config: Config = Figment::from(rocket_config).extract().unwrap();

    let tls_settings = std::sync::Arc::new(std::sync::RwLock::new(rocket::http::tls::DynamicConfig {
        certs: Vec::new(),
        key: Vec::new(),
    }));

    let (shutdown_signal_sender, mut shutdown_signal_receiver) = tokio::sync::mpsc::channel::<()>(1);

    // Create a runtime in a separate thread for the server being tested
    let join_handle = std::thread::spawn(move || {
        
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let task_handle = tokio::spawn( async {
                 rocket::custom(config).mount("/", routes![tls_test_path_state]).manage(tls_settings).launch().await.unwrap();
            });
            shutdown_signal_receiver.recv().await;
            task_handle.abort();
        });

    });

    let request_url = format!("https://localhost:{}/hello", port.unwrap());

    // Certificates are not loaded, so request should fail
    assert!(reqwest::blocking::get(&request_url).is_err());

    // Load the CA certicate for use with test client
    let mut buf = Vec::new();
    std::fs::File::open(ca_cert_path).unwrap().read_to_end(&mut buf).unwrap();
    let cert = reqwest::Certificate::from_pem(&buf).unwrap();

    let client = reqwest::blocking::Client::builder().add_root_certificate(cert).build().unwrap();
    
    // Replace with multiple tests rather than fixed wait time
    std::thread::sleep(std::time::Duration::from_secs(5));

    let response = client.get(&request_url).send();
    assert_eq!(&response.unwrap().text().unwrap(), "world");

    shutdown_signal_sender.blocking_send(()).unwrap();
    join_handle.join().unwrap();

}
