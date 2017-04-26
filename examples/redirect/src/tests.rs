use super::rocket;
use rocket::testing::MockRequest;
use rocket::Response;
use rocket::http::Method::*;
use rocket::http::Status;

macro_rules! run_test {
    ($path:expr, $test_fn:expr) => ({
        let rocket = rocket::ignite().mount("/", routes![super::root, super::login]);
        let mut request = MockRequest::new(Get, format!($path));

        $test_fn(request.dispatch_with(&rocket));
    })
}

#[test]
fn test_root() {
    run_test!("/", |mut response: Response| {
        assert!(response.body().is_none());
        assert_eq!(response.status(), Status::SeeOther);
        for h in response.headers().iter() {
            match h.name.as_str() {
                "Location" => assert_eq!(h.value, "/login"),
                "Content-Length" => assert_eq!(h.value.parse::<i32>().unwrap(), 0),
                _ => { /* let these through */ }
            }
        }
    });
}

#[test]
fn test_login() {
    run_test!("/login", |mut response: Response| {
        assert_eq!(response.body_string(),
            Some("Hi! Please log in before continuing.".to_string()));
        assert_eq!(response.status(), Status::Ok);
    });
}
