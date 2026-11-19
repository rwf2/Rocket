#![cfg(feature = "tls")]

use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

use rocket::Config;
use rocket::figment::Figment;
use rocket::fairing::AdHoc;
use rocket::tls::{TlsConfig, CipherSuite};
use rocket::tokio::sync::oneshot;
use rocket::tokio::task::{self, JoinHandle};

use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned, SupportedCipherSuite};
use rustls::crypto::ring::{self, cipher_suite};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};

fn fixture(name: &str) -> Vec<u8> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/tls");
    std::fs::read(format!("{dir}/private/{name}.pem")).unwrap()
}

#[rocket::get("/")]
fn index() -> &'static str {
    "hello"
}

#[cfg(feature = "mtls")]
#[rocket::get("/identity")]
fn identity(cert: rocket::mtls::Certificate<'_>) -> String {
    cert.subject().to_string()
}

async fn serve(tls: TlsConfig) -> (u16, rocket::Shutdown, JoinHandle<()>) {
    let (tx, rx) = oneshot::channel();
    let mut config = Config::debug_default();
    config.shutdown.ctrlc = false;
    let figment = Figment::from(config).merge(("port", 0)).merge(("tls", tls));
    let app = rocket::custom(figment).mount("/", rocket::routes![index]);
    #[cfg(feature = "mtls")]
    let app = app.mount("/", rocket::routes![identity]);
    let app = app.attach(AdHoc::on_liftoff("TLS Test", |rocket| {
        Box::pin(async move {
            let port = rocket.endpoints().find_map(|e| e.tcp()).unwrap().port();
            tx.send((port, rocket.shutdown())).unwrap();
        })
    }));

    let task = task::spawn(async move { app.launch().await.unwrap(); });
    let (port, shutdown) = rx.await.unwrap();
    (port, shutdown, task)
}

fn client(
    version: &'static rustls::SupportedProtocolVersion,
    identity: Option<&[u8]>,
) -> ClientConfig {
    let cipher_suites = vec![
        cipher_suite::TLS13_AES_256_GCM_SHA384,
        cipher_suite::TLS13_AES_128_GCM_SHA256,
        cipher_suite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
    ];

    client_with_suites(cipher_suites, version, identity)
}

fn client_with_suites(
    cipher_suites: Vec<SupportedCipherSuite>,
    version: &'static rustls::SupportedProtocolVersion,
    identity: Option<&[u8]>,
) -> ClientConfig {
    let provider = rustls::crypto::CryptoProvider {
        cipher_suites,
        ..ring::default_provider()
    };

    let mut roots = RootCertStore::empty();
    for cert in CertificateDer::pem_slice_iter(&fixture("ca_cert")) {
        roots.add(cert.unwrap()).unwrap();
    }

    let builder = ClientConfig::builder_with_provider(Arc::new(provider))
        .with_protocol_versions(&[version]).unwrap()
        .with_root_certificates(roots);

    if let Some(identity) = identity {
        let certs = CertificateDer::pem_slice_iter(identity)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        let key = PrivateKeyDer::from_pem_slice(&fixture("ed25519_key")).unwrap();
        builder.with_client_auth_cert(certs, key).unwrap()
    } else {
        builder.with_no_client_auth()
    }
}

fn connect(
    port: u16,
    config: ClientConfig,
) -> io::Result<StreamOwned<ClientConnection, TcpStream>> {
    let name = "localhost".try_into().unwrap();
    let mut conn = ClientConnection::new(Arc::new(config), name).unwrap();
    let mut tcp = TcpStream::connect(("127.0.0.1", port))?;
    tcp.set_read_timeout(Some(Duration::from_secs(5)))?;
    tcp.set_write_timeout(Some(Duration::from_secs(5)))?;
    while conn.is_handshaking() {
        conn.complete_io(&mut tcp)?;
    }

    Ok(StreamOwned::new(conn, tcp))
}

async fn request(port: u16, config: ClientConfig, path: &'static str)
    -> io::Result<String>
{
    task::spawn_blocking(move || {
        let mut stream = connect(port, config)?;
        write!(stream, "GET {path} HTTP/1.1\r\n\
            Host: localhost\r\nConnection: close\r\n\r\n")?;
        let mut response = String::new();
        stream.read_to_string(&mut response)?;
        Ok(response)
    }).await.unwrap()
}

#[rocket::async_test]
async fn tls_versions() {
    let cert = fixture("rsa_sha256_cert");
    let key = fixture("rsa_sha256_key");
    let tls = TlsConfig::from_bytes(&cert, &key);
    let (port, shutdown, server) = serve(tls).await;
    for version in [&rustls::version::TLS12, &rustls::version::TLS13] {
        let response = request(port, client(version, None), "/").await.unwrap();
        assert!(response.starts_with("HTTP/1.1 200"));
        assert!(response.ends_with("hello"));
    }

    shutdown.notify();
    server.await.unwrap();
}

#[rocket::async_test]
async fn configured_ciphers() {
    for (prefer_server, cipher) in [
        (true, rustls::CipherSuite::TLS13_AES_128_GCM_SHA256),
        (false, rustls::CipherSuite::TLS13_AES_256_GCM_SHA384),
    ] {
        let cert = fixture("rsa_sha256_cert");
        let key = fixture("rsa_sha256_key");
        let tls = TlsConfig::from_bytes(&cert, &key)
            .with_ciphers([
                CipherSuite::TLS_AES_128_GCM_SHA256,
                CipherSuite::TLS_AES_256_GCM_SHA384,
            ])
            .with_preferred_server_cipher_order(prefer_server);

        let (port, shutdown, server) = serve(tls).await;
        let stream = task::spawn_blocking(move || {
            connect(port, client(&rustls::version::TLS13, None)).unwrap()
        }).await.unwrap();

        let negotiated = stream.conn.negotiated_cipher_suite().unwrap();
        assert_eq!(negotiated.suite(), cipher);
        drop(stream);
        shutdown.notify();
        server.await.unwrap();
    }

    let suites = [
        (CipherSuite::TLS_CHACHA20_POLY1305_SHA256,
            cipher_suite::TLS13_CHACHA20_POLY1305_SHA256),
        (CipherSuite::TLS_AES_256_GCM_SHA384, cipher_suite::TLS13_AES_256_GCM_SHA384),
        (CipherSuite::TLS_AES_128_GCM_SHA256, cipher_suite::TLS13_AES_128_GCM_SHA256),
        (CipherSuite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
            cipher_suite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256),
        (CipherSuite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
            cipher_suite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384),
        (CipherSuite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
            cipher_suite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256),
    ];

    let mut configured = vec![
        CipherSuite::TLS_V13_SET.to_vec(),
        vec![CipherSuite::TLS_AES_128_GCM_SHA256],
        vec![
            CipherSuite::TLS_AES_128_GCM_SHA256,
            CipherSuite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
        ],
    ];

    // QUIC requires `TLS_AES_128_GCM_SHA256`, so HTTP/3 servers need it.
    if !cfg!(feature = "http3-preview") {
        configured.push(CipherSuite::TLS_V12_SET.to_vec());
    }

    for ciphers in configured {
        let cert = fixture("rsa_sha256_cert");
        let key = fixture("rsa_sha256_key");
        let tls = TlsConfig::from_bytes(&cert, &key).with_ciphers(ciphers.clone());
        let (port, shutdown, server) = serve(tls).await;
        for (suite, client_suite) in suites {
            let negotiated = task::spawn_blocking(move || {
                let config = client_with_suites(vec![client_suite], client_suite.version(), None);
                connect(port, config).map(|s| s.conn.negotiated_cipher_suite().unwrap().suite())
            }).await.unwrap();

            if ciphers.contains(&suite) {
                assert_eq!(negotiated.unwrap(), client_suite.suite());
            } else {
                assert!(negotiated.is_err(), "negotiated {suite:?} with {ciphers:?}");
            }
        }

        shutdown.notify();
        server.await.unwrap();
    }
}

#[rocket::async_test]
async fn tls_alpn() {
    let cert = fixture("rsa_sha256_cert");
    let key = fixture("rsa_sha256_key");
    let tls = TlsConfig::from_bytes(&cert, &key);
    let (port, shutdown, server) = serve(tls).await;
    let stream = task::spawn_blocking(move || {
        let mut config = client(&rustls::version::TLS13, None);
        config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
        connect(port, config).unwrap()
    }).await.unwrap();

    let expected = if cfg!(feature = "http2") {
        b"h2".as_slice()
    } else {
        b"http/1.1".as_slice()
    };

    assert_eq!(stream.conn.alpn_protocol(), Some(expected));
    drop(stream);
    shutdown.notify();
    server.await.unwrap();
}

#[cfg(feature = "mtls")]
#[rocket::async_test]
async fn mutual_tls() {
    use rocket::mtls::MtlsConfig;

    for mandatory in [false, true] {
        let cert = fixture("rsa_sha256_cert");
        let key = fixture("rsa_sha256_key");
        let tls = TlsConfig::from_bytes(&cert, &key)
            .with_mutual(MtlsConfig::from_bytes(&fixture("ca_cert"))
                .mandatory(mandatory));

        let (port, shutdown, server) = serve(tls).await;
        let pem = fixture("ed25519_cert");
        let config = client(&rustls::version::TLS13, Some(&pem));
        let response = request(port, config, "/identity").await.unwrap();
        assert!(response.starts_with("HTTP/1.1 200"));
        assert!(response.ends_with("C=US, ST=CA, O=Rocket, CN=localhost"));

        let response = request(port, client(&rustls::version::TLS13, None), "/")
            .await;

        if mandatory {
            assert!(response.is_err());
        } else {
            let response = response.unwrap();
            assert!(response.starts_with("HTTP/1.1 200"));
            assert!(response.ends_with("hello"));
        }

        shutdown.notify();
        server.await.unwrap();
    }
}

#[cfg(feature = "mtls")]
#[rocket::async_test]
async fn optional_mtls_rejects_invalid_certificates() {
    use rocket::mtls::MtlsConfig;

    let cert = fixture("rsa_sha256_cert");
    let key = fixture("rsa_sha256_key");
    let mutual = MtlsConfig::from_bytes(&fixture("ca_cert")).mandatory(false);
    let tls = TlsConfig::from_bytes(&cert, &key).with_mutual(mutual);

    let (port, shutdown, server) = serve(tls).await;
    for (name, cert) in [
        ("expired", include_bytes!("static/tls/expired.pem").as_slice()),
        ("server-only",
            include_bytes!("static/tls/server-only.pem").as_slice()),
        ("untrusted", include_bytes!("static/tls/untrusted.pem").as_slice()),
        ("constrained",
            include_bytes!("static/tls/constrained.pem").as_slice()),
    ] {
        let config = client(&rustls::version::TLS13, Some(cert));
        let response = request(port, config, "/").await;
        assert!(response.is_err(), "accepted {name} client certificate");
    }

    shutdown.notify();
    server.await.unwrap();
}

#[cfg(feature = "mtls")]
#[rocket::async_test]
async fn local_identity() {
    use rocket::local::asynchronous::Client;

    let app = rocket::build().mount("/", rocket::routes![identity]);
    let client = Client::debug(app).await.unwrap();
    let response = client.get("/identity")
        .identity(io::Cursor::new(fixture("ed25519_cert"))).dispatch().await;
    assert_eq!(response.status(), rocket::http::Status::Ok);
    assert_eq!(response.into_string().await.unwrap(),
        "C=US, ST=CA, O=Rocket, CN=localhost");
}
