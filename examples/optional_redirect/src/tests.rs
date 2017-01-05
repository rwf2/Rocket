use super::rocket;
use rocket::testing::MockRequest;
use rocket::http::{Method, Status};

fn test_200(uri: &str, expected_body: &str) {
    let rocket = rocket::ignite()
        .mount("/", routes![super::root, super::user, super::login]);
    let mut request = MockRequest::new(Method::Get, uri);
    let mut response = request.dispatch_with(&rocket);

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.body().and_then(|b| b.into_string()),
               Some(expected_body.to_string()));
}

fn test_303(uri: &str, expected_location: &str) {
    let rocket = rocket::ignite()
        .mount("/", routes![super::root, super::user, super::login]);
    let mut request = MockRequest::new(Method::Get, uri);
    let response = request.dispatch_with(&rocket);
    let location_headers: Vec<_> = response.header_values("Location").collect();

    assert_eq!(response.status(), Status::SeeOther);
    assert_eq!(location_headers, vec![expected_location]);
}

#[test]
fn test() {
    test_200("/users/Sergio", "Hello, Sergio!");
    test_200("/users/login",
             "Hi! That user doesn't exist. Maybe you need to log in?");
}

#[test]
fn test_redirects() {
    test_303("/", "/users/login");
    test_303("/users/unknown", "/users/login");
}
