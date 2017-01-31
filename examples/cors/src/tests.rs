use super::rocket;
use rocket::testing::MockRequest;
use rocket::http::Header;
use rocket::http::Method::*;

#[test]
fn user() {
    let rocket = rocket::ignite().mount("/", routes![super::hello]);
    let mut req = MockRequest::new(Get, "/hello");
    let mut response = req.dispatch_with(&rocket);

    let body_str = response.body().and_then(|body| body.into_string());
    let values: Vec<_> = response.header_values("Access-Control-Allow-Origin").collect();
    assert_eq!(values, vec!["*"]);
    assert_eq!(body_str, Some("Hello there!".to_string()));
}
