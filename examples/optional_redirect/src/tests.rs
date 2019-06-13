use rocket::local::Client;
use rocket::http::Status;

fn client() -> Client {
    let rocket = rocket::ignite()
        .mount("/", routes![super::root, super::user, super::login]);
    Client::new(rocket).unwrap()

}

fn test_200(uri: &str, expected_body: &str) {
    let client = client();
    let mut response = client.get(uri).dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.body_string(), Some(expected_body.to_string()));
}

fn test_303(uri: &str, expected_location: &str) {
    let client = client();
    let response = client.get(uri).dispatch();
    let location_headers: Vec<_> = response.headers().get("Location").collect();
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
