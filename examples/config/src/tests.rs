use super::rocket;
use rocket::testing::MockRequest;
use rocket::http::Method;

#[test]
fn test_hello_world() {
    let rocket = rocket::ignite().mount("/hello", routes![super::hello]);
    let mut request = MockRequest::new(Method::Get, "/hello");
    let mut response = request.dispatch_with(&rocket);

    assert_eq!(response.body().and_then(|b| b.into_string()),
               Some("Hello, world!".to_string()));
}
