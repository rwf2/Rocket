use super::rocket;
use rocket::testing::MockRequest;
use rocket::http::Method;
use rocket::http::Status;

fn test(method: Method, status: Status, body_prefix: Option<&str>) {
    let rocket = rocket::ignite()
        .mount("/", routes![super::index, super::put]);

    let mut req = MockRequest::new(method, "/");
    let mut response = req.dispatch_with(&rocket);

    assert_eq!(response.status(), status);
    if let Some(expected_body_string) = body_prefix {
        let body_str = response.body_string().unwrap();
        assert!(body_str.starts_with(expected_body_string));
    }
}

#[test]
fn hello_world_alt_methods() {
    test(Method::Get, Status::Ok, Some("<!DOCTYPE html>"));
    test(Method::Put, Status::Ok, Some("Hello, PUT request!"));
    test(Method::Post, Status::NotFound, None);
}
