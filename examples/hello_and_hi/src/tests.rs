use super::rocket;
use rocket::http::Status;
use rocket::local::Client;

fn client() -> Client {
    Client::new(
        rocket::ignite()
            .mount("/hello", routes![super::world])
            .mount("/hi", routes![super::japan]),
    )
    .unwrap()
}

fn test_ok(uri: &str, expected: &str) {
    let client = client();
    assert_eq!(
        client.get(uri).dispatch().body_string(),
        Some(expected.to_string())
    );
}

#[test]
fn test_hello() {
    test_ok("/hello", "hello, world!");
}

#[test]
fn test_hi() {
    test_ok("/hi", "hi, japan!");
}

#[test]
fn test_not_found() {
    let test_none = client().get("/").dispatch().status();
    let test_invalid = client().get("/invalid").dispatch().status();
    assert_eq!(test_none, Status::NotFound);
    assert_eq!(test_invalid, Status::NotFound);
}
