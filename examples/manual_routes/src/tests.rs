use super::*;
use rocket::local::blocking::Client;
use rocket::http::{ContentType, Status};

fn test(uri: &str, content_type: ContentType, status: Status, body: String) {
    let client = Client::tracked(rocket()).unwrap();
    let response = client.get(uri).header(content_type).dispatch();
    assert_eq!(response.status(), status);
    assert_eq!(response.into_string(), Some(body));
}

#[test]
fn test_forward() {
    test("/", ContentType::Plain, Status::Ok, "Hello!".to_string());
}

#[test]
fn test_name() {
    for &name in &[("John"), ("Mike"), ("Angela")] {
        let uri = format!("/hello/{}", name);
        test(&uri, ContentType::Plain, Status::Ok, name.to_string());
    }
}

#[test]
fn test_echo() {
    let uri = format!("/echo/echo%20this%20text");
    test(&uri, ContentType::Plain, Status::Ok, "echo this text".into());
}

#[test]
fn test_upload() {
    let client = Client::tracked(rocket()).unwrap();
    let expected_body = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, \
                         sed do eiusmod tempor incididunt ut labore et dolore \
                         magna aliqua".to_string();

    // Upload the body.
    let response = client.post("/upload")
        .header(ContentType::Plain)
        .body(&expected_body)
        .dispatch();

    assert_eq!(response.status(), Status::Ok);

    // Ensure we get back the same body.
    let response = client.get("/upload").dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.into_string(), Some(expected_body));
}

#[test]
fn test_not_found() {
    let uri = "/wrong_address";
    let expected_body = format!("Couldn't find: {}", uri);
    test(uri, ContentType::Plain, Status::NotFound, expected_body);
}
