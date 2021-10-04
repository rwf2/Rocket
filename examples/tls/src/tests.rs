use rocket::local::blocking::Client;

#[test]
fn hello_mutual() {
    let client = Client::tracked(super::rocket()).unwrap();
    let response = client
        .get("/")
        .identity("private/rsa_sha256_cert.pem".into())
        .dispatch();
    assert_eq!(response.into_string(), Some("Hello! Here's what we know: [301438261598342027628437991669560131935683596135] C=US, ST=CA, O=Rocket, CN=localhost".into()));
}

#[test]
fn hello_world() {
    let client = Client::tracked(super::rocket()).unwrap();
    let response = client.get("/").dispatch();
    assert_eq!(response.into_string(), Some("Hello, world!".into()));
}
