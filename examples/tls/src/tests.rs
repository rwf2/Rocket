use std::fs::{self, File};

use rocket::http::{CookieJar, Cookie};
use rocket::local::blocking::Client;
use rocket::fs::relative;

#[get("/cookie")]
fn cookie(jar: &CookieJar<'_>) {
    jar.add(("k1", "v1"));
    jar.add_private(("k2", "v2"));

    jar.add(Cookie::build(("k1u", "v1u")).secure(false));
    jar.add_private(Cookie::build(("k2u", "v2u")).secure(false));
}

#[test]
fn hello_mutual() {
    let client = Client::tracked_secure(super::rocket()).unwrap();
    let cert_paths = fs::read_dir(relative!("private")).unwrap()
        .map(|entry| entry.unwrap().path().to_string_lossy().into_owned())
        .filter(|path| path.ends_with("_cert.pem") && !path.ends_with("ca_cert.pem"));

    for path in cert_paths {
        let response = client.get("/")
            .identity(File::open(&path).unwrap())
            .dispatch();

        let response = response.into_string().unwrap();
        let subject = response.split(']').nth(1).unwrap().trim();
        assert_eq!(subject, "C=US, ST=CA, O=Rocket, CN=localhost");
    }
}

#[test]
fn secure_cookies() {
    let rocket = super::rocket().mount("/", routes![cookie]);
    let client = Client::tracked_secure(rocket).unwrap();

    let response = client.get("/cookie").dispatch();
    let c1 = response.cookies().get("k1").unwrap();
    let c2 = response.cookies().get_private("k2").unwrap();
    let c3 = response.cookies().get("k1u").unwrap();
    let c4 = response.cookies().get_private("k2u").unwrap();

    assert_eq!(c1.secure(), Some(true));
    assert_eq!(c2.secure(), Some(true));
    assert_ne!(c3.secure(), Some(true));
    assert_ne!(c4.secure(), Some(true));
}

#[test]
fn insecure_cookies() {
    let rocket = super::rocket().mount("/", routes![cookie]);
    let client = Client::tracked(rocket).unwrap();

    let response = client.get("/cookie").dispatch();
    let c1 = response.cookies().get("k1").unwrap();
    let c2 = response.cookies().get_private("k2").unwrap();
    let c3 = response.cookies().get("k1u").unwrap();
    let c4 = response.cookies().get_private("k2u").unwrap();

    assert_eq!(c1.secure(), None);
    assert_eq!(c2.secure(), None);
    assert_eq!(c3.secure(), None);
    assert_eq!(c4.secure(), None);
}

#[test]
fn hello_world() {
    use rocket::listener::DefaultListener;
    use rocket::config::{Config, SecretKey};
    use rustls::crypto::aws_lc_rs;

    let mut profiles = vec![
        "rsa_sha256",
        "ecdsa_nistp256_sha256_pkcs8",
        "ecdsa_nistp384_sha384_pkcs8",
        "ecdsa_nistp256_sha256_sec1",
        "ecdsa_nistp384_sha384_sec1",
        "ed25519",
    ];

    for use_aws_lc in [false, true] {
        if use_aws_lc {
            let crypto_provider = aws_lc_rs::default_provider();
            crypto_provider.install_default().unwrap();

            profiles.push("ecdsa_nistp521_sha512_pkcs8");
        }

        for profile in &profiles {
            let config = Config {
                secret_key: SecretKey::generate().unwrap(),
                ..Config::debug_default()
            };

            let figment = Config::figment().merge(config).select(profile);
            let client = Client::tracked_secure(super::rocket().configure(figment)).unwrap();
            let response = client.get("/").dispatch();
            assert_eq!(response.into_string().unwrap(), "Hello, world!");

            let figment = client.rocket().figment();
            let listener: DefaultListener = figment.extract().unwrap();
            assert_eq!(figment.profile(), profile);
            listener.tls.as_ref().unwrap().validate().expect("valid TLS config");
        }
    }
}
