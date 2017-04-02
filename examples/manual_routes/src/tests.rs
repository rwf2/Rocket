use super::*;
use rocket::testing::MockRequest;
use rocket::http::{ContentType, Status};
use rocket::http::Method::*;

fn test(uri: &str, content_type: ContentType, status: Status, body: String) {
    let rocket = rocket();
    let mut request = MockRequest::new(Get, uri).header(content_type);
    let mut response = request.dispatch_with(&rocket);

    assert_eq!(response.status(), status);
    assert_eq!(response.body().and_then(|b| b.into_string()), Some(body));
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
    let echo = "echo text";
    let uri = format!("/echo:echo text");
    test(&uri, ContentType::Plain, Status::Ok, echo.to_string());
}

#[test]
fn test_upload() {
    let rocket = rocket();
    let expected_body = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, \
                         sed do eiusmod tempor incididunt ut labore et dolore \
                         magna aliqua".to_string();

    // Upload the body.
    let mut request = MockRequest::new(Post, "/upload")
        .header(ContentType::Plain)
        .body(&expected_body);
    let response = request.dispatch_with(&rocket);
    assert_eq!(response.status(), Status::Ok);

    // Ensure we get back the same body.
    let mut request = MockRequest::new(Get, "/upload");
    let mut response = request.dispatch_with(&rocket);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.body().and_then(|b| b.into_string()), Some(expected_body));
}

#[test]
fn test_not_found() {
    let uri = "/wrong_address";
    let expected_body = format!("Couldn't find: {}", uri);
    test(uri, ContentType::Plain, Status::NotFound, expected_body);
}
