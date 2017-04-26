use super::rocket;
use rocket::testing::MockRequest;
use rocket::http::{Method, Status};

#[test]
fn test_200() {
    let rocket = rocket::ignite().mount("/", routes![super::user]);
    let mut request = MockRequest::new(Method::Get, "/users/Sergio");
    let mut response = request.dispatch_with(&rocket);

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.body_string(), Some("Hello, Sergio!".into()));
}

#[test]
fn test_404() {
    let rocket = rocket::ignite().mount("/", routes![super::user]);
    let mut request = MockRequest::new(Method::Get, "/users/unknown");
    let response = request.dispatch_with(&rocket);

    // Only test the status because the body is the default 404.
    assert_eq!(response.status(), Status::NotFound);
}
