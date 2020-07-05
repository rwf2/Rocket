use super::rocket;
use rocket::local::blocking::Client;
use rocket::http::{ContentType, Status};

fn test_login<T>(user: &str, pass: &str, age: &str, status: Status, body: T)
    where T: Into<Option<&'static str>> + Send
{
    let client = Client::new(rocket()).unwrap();
    let query = format!("username={}&password={}&age={}", user, pass, age);
    let response = client.post("/login")
        .header(ContentType::Form)
        .body(&query)
        .dispatch();

    assert_eq!(response.status(), status);
    if let Some(expected_str) = body.into() {
        let body_str = response.into_string();
        assert!(body_str.map_or(false, |s| s.contains(expected_str)));
    }
}

#[test]
fn test_good_login() {
    test_login("Sergio", "password", "30", Status::SeeOther, None);
}

#[test]
fn test_invalid_user() {
    test_login("-1", "password", "30", Status::Ok, "Unrecognized user");
    test_login("Mike", "password", "30", Status::Ok, "Unrecognized user");
}

#[test]
fn test_invalid_password() {
    test_login("Sergio", "password1", "30", Status::Ok, "Wrong password!");
    test_login("Sergio", "ok", "30", Status::Ok, "Password is invalid: too short!");
}

#[test]
fn test_invalid_age() {
    test_login("Sergio", "password", "20", Status::Ok, "must be at least 21.");
    test_login("Sergio", "password", "-100", Status::Ok, "must be at least 21.");
    test_login("Sergio", "password", "hi", Status::Ok, "value is not a number");
}

fn check_bad_form(form_str: &str, status: Status) {
    let client = Client::new(rocket()).unwrap();
    let response = client.post("/login")
        .header(ContentType::Form)
        .body(form_str)
        .dispatch();

    assert_eq!(response.status(), status);
}

#[test]
fn test_bad_form_abnromal_inputs() {
    check_bad_form("&&&===&", Status::BadRequest);
    check_bad_form("&&&=hi==&", Status::BadRequest);
}

#[test]
fn test_bad_form_missing_fields() {
    let bad_inputs: [&str; 8] = [
        "&",
        "=",
        "username=Sergio",
        "password=pass",
        "age=30",
        "username=Sergio&password=pass",
        "username=Sergio&age=30",
        "password=pass&age=30"
    ];

    for bad_input in bad_inputs.iter() {
        check_bad_form(bad_input, Status::UnprocessableEntity);
    }
}

#[test]
fn test_bad_form_additional_fields() {
    check_bad_form("username=Sergio&password=pass&age=30&addition=1",
                   Status::UnprocessableEntity);
}
