use rocket::testing::MockRequest;
use rocket::http::Method::*;
use rocket::http::{ContentType, Status};

use super::rocket;

fn test_login<T>(user: &str, pass: &str, age: &str, status: Status, body: T)
    where T: Into<Option<&'static str>>
{
    let rocket = rocket();
    let query = &format!("username={}&password={}&age={}", user, pass, age);

    let mut req = MockRequest::new(Post, "/login")
        .header(ContentType::Form)
        .body(&query);

    let mut response = req.dispatch_with(&rocket);
    assert_eq!(response.status(), status);
    let body_str = response.body().and_then(|body| body.into_string());
    if let Some(string) = body.into() {
        assert!(body_str.map_or(true, |s| s.contains(string)));
    }
}

const OK: Status = self::Status::Ok;

#[test]
fn test_good_login() {
    test_login("Sergio", "password", "30", Status::SeeOther, None);
}

#[test]
fn test_invalid_user() {
    test_login("-1", "password", "30", OK, "Unrecognized user");
    test_login("Mike", "password", "30", OK, "Unrecognized user");
}

#[test]
fn test_invalid_password() {
    test_login("Sergio", "password1", "30", OK, "Wrong password!");
    test_login("Sergio", "ok", "30", OK, "Password is invalid: Too short!");
}

#[test]
fn test_invalid_age() {
    test_login("Sergio", "password", "20", OK, "Must be at least 21.");
    test_login("Sergio", "password", "-100", OK, "Must be at least 21.");
    test_login("Sergio", "password", "hi", OK, "Age value is not a number");
}

fn check_bad_form(form_str: &str, status: Status) {
    let rocket = rocket();
    let mut req = MockRequest::new(Post, "/login")
        .header(ContentType::Form)
        .body(form_str);

    let response = req.dispatch_with(&rocket);
    assert_eq!(response.status(), status);
}

#[test]
fn test_bad_form() {
    check_bad_form("&", Status::BadRequest);
    check_bad_form("=", Status::BadRequest);
    check_bad_form("&&&===&", Status::BadRequest);

    check_bad_form("username=Sergio", Status::UnprocessableEntity);
    check_bad_form("username=Sergio&", Status::UnprocessableEntity);
    check_bad_form("username=Sergio&pass=something", Status::UnprocessableEntity);
    check_bad_form("user=Sergio&password=something", Status::UnprocessableEntity);
    check_bad_form("password=something", Status::UnprocessableEntity);
}
