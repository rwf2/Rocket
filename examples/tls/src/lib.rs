#[cfg(test)]
mod tests;
mod redirector;

#[macro_use]
extern crate rocket;

use rocket::mtls::Certificate;
use rocket::listener::Endpoint;
use rocket::local::blocking::Client;

pub const DEFAULT_PROFILES: &[&str] = &[
    "rsa_sha256",
    "ecdsa_nistp256_sha256_pkcs8",
    "ecdsa_nistp384_sha384_pkcs8",
    "ecdsa_nistp256_sha256_sec1",
    "ecdsa_nistp384_sha384_sec1",
    "ed25519",
];

pub fn validate_profiles(profiles: &[&str]) {
    use rocket::listener::DefaultListener;
    use rocket::config::{Config, SecretKey};

    for profile in profiles {
        let config = Config {
            secret_key: SecretKey::generate().unwrap(),
            ..Config::debug_default()
        };

        let figment = Config::figment().merge(config).select(profile);
        let client = Client::tracked_secure(crate::rocket().configure(figment)).unwrap();
        let response = client.get("/").dispatch();
        assert_eq!(response.into_string().unwrap(), "Hello, world!");

        let figment = client.rocket().figment();
        let listener: DefaultListener = figment.extract().unwrap();
        assert_eq!(figment.profile(), profile);
        listener.tls.as_ref().unwrap().validate().expect("valid TLS config");
    }
}

#[get("/")]
fn mutual(cert: Certificate<'_>) -> String {
    format!("Hello! Here's what we know: [{}] {}", cert.serial(), cert.subject())
}

#[get("/", rank = 2)]
fn hello(endpoint: Option<&Endpoint>) -> String {
    match endpoint {
        Some(endpoint) => format!("Hello, {endpoint}!"),
        None => "Hello, world!".into(),
    }
}

#[launch]
pub fn rocket() -> _ {
    // See `Rocket.toml` and `Cargo.toml` for TLS configuration.
    // Run `./private/gen_certs.sh` to generate a CA and key pairs.
    rocket::build()
        .mount("/", routes![hello, mutual])
        .attach(crate::redirector::Redirector::on(3000))
}
