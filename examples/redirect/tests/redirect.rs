#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate redirect;

use rocket::testing::MockRequest;
use rocket::Response;
use rocket::http::Method::*;
use rocket::http::Status;

macro_rules! test_get_path {
    ($path:expr, $test_fn:expr) => ({
        let rocket = rocket::ignite().mount("/", routes![redirect::root, redirect::login]);
        let mut request = MockRequest::new(Get, format!($path));

        $test_fn(request.dispatch_with(&rocket));
    })
}

#[test]
fn test_root_response_status() {
    test_get_path!("/", |response: Response| {
        assert_eq!(Status::SeeOther, response.status());
    });
}

#[test]
fn test_root_redirect_headers() {
    test_get_path!("/", |response: Response| {
        let headers = response.headers();

        for h in headers {
            if h.name == "Location" {
                assert_eq!("/login", h.value);
            } else if h.name == "Content-Length" {
                assert_eq!("0", h.value);
            }
        }
    });
}

#[test]
fn test_root_response_body() {
    test_get_path!("/", |mut response: Response| {
        let body_string = response.body().and_then(|body| body.into_string());

        assert_eq!(None, body_string);
    });
}

#[test]
fn test_login_response_status() {
    test_get_path!("/login", |response: Response| {
        assert_eq!(Status::Ok, response.status());
    });
}

#[test]
fn test_login_response_body() {
    test_get_path!("/login", |mut response: Response| {
        let body_string = response.body().and_then(|body| body.into_string());

        assert_eq!(Some("Hi! Please log in before continuing.".to_string()), body_string);
    });
}

