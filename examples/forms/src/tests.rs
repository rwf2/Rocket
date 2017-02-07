use rocket::testing::MockRequest;
use rocket::http::Method::*;
use rocket::http::{ContentType, Status};

use super::rocket;

fn test_login(username: &str, password: &str, age: isize, status: Status,
              body: Option<&'static str>) {
    let rocket = rocket();
    let mut req = MockRequest::new(Post, "/login")
        .header(ContentType::Form)
        .body(&format!("username={}&password={}&age={}", username, password, age));

    let mut response = req.dispatch_with(&rocket);
    let body_str = response.body().and_then(|body| body.into_string());

    println!("Checking: {:?}/{:?}/{:?}/{:?}", username, password, age, body_str);
    assert_eq!(response.status(), status);

    if let Some(string) = body {
        assert!(body_str.map_or(true, |s| s.contains(string)));
    }
}

#[test]
fn test_good_login() {
    test_login("Sergio", "password", 30, Status::SeeOther, None);
}

const OK: Status = self::Status::Ok;

#[test]
fn test_bad_login() {
    test_login("Sergio", "password", 20, OK, Some("Sorry, 20 is too young!"));
    test_login("Sergio", "password", 200, OK, Some("Are you sure you're 200?"));
    test_login("Sergio", "jk", -100, OK, Some("'-100' is not a valid integer."));
    test_login("Sergio", "ok", 30, OK, Some("Wrong password!"));
    test_login("Mike", "password", 30, OK, Some("Unrecognized user, 'Mike'."));
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
