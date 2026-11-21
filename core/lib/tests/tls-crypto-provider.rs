#![cfg(feature = "tls")]

mod server;

use std::io;
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

use rocket::Config;
use rocket::figment::Figment;
use rocket::tls::{TlsConfig, CipherSuite};
use rocket::tokio::task;

use rustls::{ClientConfig, ClientConnection, RootCertStore, SupportedCipherSuite};
use rustls::crypto::CryptoProvider;
use rustls::crypto::ring::{self, cipher_suite};
use rustls::pki_types::{CertificateDer, pem::PemObject};

use server::Server;

fn fixture(name: &str) -> Vec<u8> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/tls");
    std::fs::read(format!("{dir}/private/{name}.pem")).unwrap()
}

fn negotiate(
    port: u16,
    version: &'static rustls::SupportedProtocolVersion,
    cipher_suites: Vec<SupportedCipherSuite>,
) -> io::Result<rustls::CipherSuite> {
    let provider = CryptoProvider { cipher_suites, ..ring::default_provider() };
    let mut roots = RootCertStore::empty();
    for cert in CertificateDer::pem_slice_iter(&fixture("ca_cert")) {
        roots.add(cert.unwrap()).unwrap();
    }

    let config = ClientConfig::builder_with_provider(Arc::new(provider))
        .with_protocol_versions(&[version]).unwrap()
        .with_root_certificates(roots)
        .with_no_client_auth();

    let name = "localhost".try_into().unwrap();
    let mut conn = ClientConnection::new(Arc::new(config), name).unwrap();
    let mut tcp = TcpStream::connect(("127.0.0.1", port))?;
    tcp.set_read_timeout(Some(Duration::from_secs(5)))?;
    tcp.set_write_timeout(Some(Duration::from_secs(5)))?;
    while conn.is_handshaking() {
        conn.complete_io(&mut tcp)?;
    }

    Ok(conn.negotiated_cipher_suite().unwrap().suite())
}

#[rocket::async_test]
async fn installed_provider_replaces_configured_ciphers() {
    // QUIC requires `TLS13_AES_128_GCM_SHA256`, so HTTP/3 servers need it.
    let installed = vec![
        cipher_suite::TLS13_AES_128_GCM_SHA256,
        cipher_suite::TLS13_CHACHA20_POLY1305_SHA256,
        cipher_suite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
    ];

    let provider = CryptoProvider {
        cipher_suites: installed.clone(),
        ..ring::default_provider()
    };

    provider.install_default().unwrap();
    let configured = [
        CipherSuite::TLS_AES_256_GCM_SHA384,
        CipherSuite::TLS_CHACHA20_POLY1305_SHA256,
    ];

    for (prefer_server, cipher) in [
        (true, rustls::CipherSuite::TLS13_AES_128_GCM_SHA256),
        (false, rustls::CipherSuite::TLS13_CHACHA20_POLY1305_SHA256),
    ] {
        let cert = fixture("rsa_sha256_cert");
        let key = fixture("rsa_sha256_key");
        let tls = TlsConfig::from_bytes(&cert, &key)
            .with_ciphers(configured)
            .with_preferred_server_cipher_order(prefer_server);

        let mut config = Config::debug_default();
        config.shutdown.ctrlc = false;
        let app = rocket::custom(Figment::from(config).merge(("tls", tls)));
        let tls: TlsConfig = app.figment().extract_inner("tls").unwrap();
        assert!(tls.ciphers().eq(configured));

        let server = Server::launch(app).await;
        let port = server.addr.port();
        let negotiated = task::spawn_blocking(move || {
            let suites = vec![
                cipher_suite::TLS13_CHACHA20_POLY1305_SHA256,
                cipher_suite::TLS13_AES_128_GCM_SHA256,
            ];

            negotiate(port, &rustls::version::TLS13, suites).unwrap()
        }).await.unwrap();

        assert_eq!(negotiated, cipher);
        for suite in [
            cipher_suite::TLS13_CHACHA20_POLY1305_SHA256,
            cipher_suite::TLS13_AES_256_GCM_SHA384,
            cipher_suite::TLS13_AES_128_GCM_SHA256,
            cipher_suite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
            cipher_suite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
            cipher_suite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
        ] {
            let negotiated = task::spawn_blocking(move || {
                negotiate(port, suite.version(), vec![suite])
            }).await.unwrap();

            if installed.contains(&suite) {
                assert_eq!(negotiated.unwrap(), suite.suite());
            } else {
                assert!(negotiated.is_err(), "negotiated {:?}", suite.suite());
            }
        }

        server.shutdown().await;
    }
}
