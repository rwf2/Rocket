use rocket::{http::tls::ClientCertificate, local::blocking::Client};

#[test]
fn get_index_without_auth() {
    let client = Client::tracked(super::rocket()).unwrap();
    let response = client.get("/").dispatch();
    assert_eq!(response.into_string(), Some("Hello, anonymous user".into()));
}

#[test]
fn get_secret_without_auth() {
    let client = Client::tracked(super::rocket()).unwrap();
    let response = client.get("/secret").dispatch();
    assert_eq!(response.status().code, 404);
}

fn get_cert() -> ClientCertificate {
    ClientCertificate::generate(&["mary"])
}

#[test]
fn get_index_with_auth() {
    let client = Client::tracked(super::rocket()).unwrap();
    let response = client.get("/").client_certificate(get_cert()).dispatch();
    assert_eq!(
        response.into_string(),
        Some("Hello, [\"rcgen self signed cert\"] []".into())
    );
}

#[test]
fn get_secret_with_auth() {
    let client = Client::tracked(super::rocket()).unwrap();
    let response = client.get("/secret").client_certificate(get_cert()).dispatch();
    assert_eq!(response.status().code, 200);
    assert_eq!(response.into_string(), Some("secret stuff".into()));
}
