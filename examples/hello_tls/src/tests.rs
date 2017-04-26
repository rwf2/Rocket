use super::rocket;
use rocket::testing::MockRequest;
use rocket::http::Method::*;

#[test]
fn hello_world() {
    let rocket = rocket::ignite().mount("/", routes![super::hello]);
    let mut req = MockRequest::new(Get, "/");
    let mut response = req.dispatch_with(&rocket);

    assert_eq!(response.body_string(), Some("Hello, world!".into()));
}
