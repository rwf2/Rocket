use crate::prelude::*;

fn test_mtls(mandatory: bool) -> Result<()> {
    let server = spawn!(mandatory: bool => {
        let mtls_config = format!(r#"
            [default.tls.mutual]
            ca_certs = "{{ROCKET}}/examples/tls/private/ca_cert.pem"
            mandatory = {mandatory}
        "#);

        #[get("/")]
        fn hello(cert: rocket::mtls::Certificate<'_>) -> String {
            format!("{}:{}[{}] {}", cert.serial(), cert.version(), cert.issuer(), cert.subject())
        }

        #[get("/", rank = 2)]
        fn hi() -> &'static str {
            "Hello!"
        }

        Rocket::tls_default()
            .reconfigure_with_toml(&mtls_config)
            .mount("/", routes![hello, hi])
    })?;

    let pem = read("{ROCKET}/examples/tls/private/client.pem")?;
    let client: Client = Client::build()
        .identity(reqwest::Identity::from_pem(&pem)?)
        .try_into()?;

    let response = client.get(&server, "/")?.send()?.text()?;
    let (_key_hash, subject) = response.split_once(":2").unwrap();
    assert_eq!(subject,
        "[C=US, ST=CA, O=Rocket CA, CN=Rocket Root CA] \
            C=US, ST=California, L=Silicon Valley, O=Rocket, \
            CN=Rocket TLS Example, Email=example@rocket.local");

    let client = Client::default();
    let response = client.get(&server, "/")?.send();
    if mandatory {
        assert!(response.unwrap_err().is_request());
    } else {
        assert_eq!(response?.text()?, "Hello!");
    }

    Ok(())
}

register!(test_mtls(mandatory: true));
register!(test_mtls(mandatory: false));
