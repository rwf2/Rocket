use super::rocket;
use rocket::testing::MockRequest;
use rocket::http::Method;
use rocket::http::Status;

#[test]
fn hello_world_alt_methods() {
    test(Method::Get, Status::Ok, Some("<!DOCTYPE html>"));
    test(Method::Put, Status::Ok, Some("Hello, PUT request!"));
    test(Method::Post, Status::NotFound, None);
}

fn test(
    method: Method,
    status: Status,
    body_prefix: Option<&str>
) {
    let rocket = rocket::ignite()
        .mount("/", routes![super::index, super::put]);

    let mut req = MockRequest::new(method, "/");
    let mut response = req.dispatch_with(&rocket);

    assert_eq!(response.status(), status);
    body_prefix.map(|expected_body_string| {
        let body_str = response.body().and_then(|body| body.into_string()).unwrap();
        let body_starts_correctly = body_str.starts_with(expected_body_string);
        assert!(body_starts_correctly);
    });
}

