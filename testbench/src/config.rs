use rocket::{Build, Rocket};
use rustls_pki_types::{CertificateDer, pem::PemObject};

use testbench::{Result, Error};

pub static DEFAULT_CONFIG: &str = r#"
    [default]
    address = "tcp:127.0.0.1"
    workers = 2
    port = 0
    cli_colors = false
    log_level = "debug"
    secret_key = "itlYmFR2vYKrOmFhupMIn/hyB6lYCCTXz4yaQX89XVg="

    [default.shutdown]
    grace = 1
    mercy = 1
"#;

pub static TLS_CONFIG: &str = r#"
    [default.tls]
    certs = "{ROCKET}/examples/tls/private/rsa_sha256_cert.pem"
    key = "{ROCKET}/examples/tls/private/rsa_sha256_key.pem"
"#;

pub trait RocketExt {
    fn default() -> Self;
    fn tls_default() -> Self;
    fn reconfigure_with_toml(self, toml: &str) -> Self;
}

impl RocketExt for Rocket<Build> {
    fn default() -> Self {
        rocket::build().reconfigure_with_toml(DEFAULT_CONFIG)
    }

    fn tls_default() -> Self {
        rocket::build()
            .reconfigure_with_toml(DEFAULT_CONFIG)
            .reconfigure_with_toml(TLS_CONFIG)
    }

    fn reconfigure_with_toml(self, toml: &str) -> Self {
        use rocket::figment::{Figment, providers::{Format, Toml}};

        let toml = toml.replace("{ROCKET}", rocket::fs::relative!("../"));
        let config = Figment::from(self.figment())
            .merge(Toml::string(&toml).nested());

        self.reconfigure(config)
    }
}

pub fn read(path: &str) -> Result<Vec<u8>> {
    let path = path.replace("{ROCKET}", rocket::fs::relative!("../"));
    Ok(std::fs::read(path)?)
}

pub fn cert(path: &str) -> Result<Vec<u8>> {
    let cert = CertificateDer::pem_slice_iter(&read(path)?).last();
    Ok(cert.ok_or(Error::MissingCertificate)??.to_vec())
}
