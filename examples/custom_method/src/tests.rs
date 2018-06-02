use super::rocket;
use rocket::local::Client;
use rocket::http::Method;

#[test]
fn hello_world() {
    let rocket = rocket::ignite().mount("/", routes![super::hello]);
    let client = Client::new(rocket).unwrap();
    let mut response = client.req(Method::Extension("CUSTOM".to_string()), "/").dispatch();
    assert_eq!(response.body_string(), Some("Hello, world!".into()));
}
