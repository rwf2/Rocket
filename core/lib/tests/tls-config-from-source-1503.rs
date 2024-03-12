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

    let cert_path = relative!("../../examples/tls/private/cert.pem");
    let key_path = relative!("../../examples/tls/private/key.pem");

    let rocket_config = Config {
        tls: Some(TlsConfig::from_paths(cert_path, key_path)),
        ..Default::default()
    };

    let config: Config = Figment::from(rocket_config).extract().unwrap();
    let tls = config.tls.expect("have TLS config");
    assert_eq!(tls.certs().unwrap_left(), cert_path);
    assert_eq!(tls.key().unwrap_left(), key_path);
}

#[test]
fn tls_server_operation() {
    use std::io::Read;
    
    use rocket::{get, routes};
    use rocket::config::{Config, TlsConfig};
    use rocket::figment::Figment;

    let cert_path =    relative!("../../examples/tls/private/rsa_sha256_cert.pem");
    let key_path =     relative!("../../examples/tls/private/rsa_sha256_key.pem");
    let ca_cert_path = relative!("../../examples/tls/private/ca_cert.pem");

    println!("{cert_path:?}");

    let port = {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("creating listener");
        listener.local_addr().expect("getting listener's port").port()
    };

    let rocket_config = Config { 
        port,
        tls: Some(TlsConfig::from_paths(cert_path, key_path)),
        ..Default::default()
    };
    let config: Config = Figment::from(rocket_config).extract().expect("creating config");
    let (shutdown_signal_sender, mut shutdown_signal_receiver) = tokio::sync::mpsc::channel::<()>(1);

    // Create a runtime in a separate thread for the server being tested
    let join_handle = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();

        #[get("/hello")]
        fn tls_test_get()  -> &'static str {
            "world"
        }

        rt.block_on(async {
            let task_handle = tokio::spawn( async {
                 rocket::custom(config)
                    .mount("/", routes![tls_test_get])
                    .launch().await.unwrap();
            });
            shutdown_signal_receiver.recv().await;
            task_handle.abort();
        });
    });

    let request_url = format!("https://localhost:{}/hello", port);

    // CA certificate is not loaded, so request should fail
    assert!(reqwest::blocking::get(&request_url).is_err());

    // Load the CA certicate for use with test client
    let cert = {
        let mut buf = Vec::new();
        std::fs::File::open(ca_cert_path).expect("open ca_certs")
            .read_to_end(&mut buf).expect("read ca_certs");
        reqwest::Certificate::from_pem(&buf).expect("create certificate")
    };
    let client = reqwest::blocking::Client::builder().add_root_certificate(cert).build().expect("build client");

    let response = client.get(&request_url).send().expect("https request");
    assert_eq!(&response.text().unwrap(), "world");

    shutdown_signal_sender.blocking_send(()).expect("signal shutdown");
    join_handle.join().expect("join thread");
}
