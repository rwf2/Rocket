use super::rocket;
use rocket::testing::MockRequest;
use rocket::http::Method::*;
use rocket::http::Status;

fn test(uri: &str, expected: String) {
    let rocket = rocket::ignite().mount("/", routes![super::hello, super::hi]);
    let mut req = MockRequest::new(Get, uri);
    let mut response = req.dispatch_with(&rocket);
    assert_eq!(response.body_string(), Some(expected));
}

fn test_404(uri: &str) {
    let rocket = rocket::ignite().mount("/", routes![super::hello, super::hi]);
    let mut req = MockRequest::new(Get, uri);
    let response = req.dispatch_with(&rocket);
    assert_eq!(response.status(), Status::NotFound);
}

#[test]
fn test_hello() {
    for &(name, age) in &[("Mike", 22), ("Michael", 80), ("A", 0), ("a", 127)] {
        test(&format!("/hello/{}/{}", name, age),
            format!("Hello, {} year old named {}!", age, name));
    }
}

#[test]
fn test_failing_hello() {
    test_404("/hello/Mike/1000");
    test_404("/hello/Mike/-129");
    test_404("/hello/Mike/-1");
}

#[test]
fn test_hi() {
    for name in &["Mike", "A", "123", "hi", "c"] {
        test(&format!("/hello/{}", name), name.to_string());
    }
}
