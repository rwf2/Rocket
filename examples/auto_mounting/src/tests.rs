use super::rocket;
use rocket::local::Client;
use rocket::http::Status;

#[test]
fn auto_mount() {
    let rocket = rocket::ignite().auto_mount();
    let client = Client::new(rocket).unwrap();

    let mut response = client.get("/").dispatch();
    assert_eq!(response.body_string(), Some("Hello, world!".into()));

    let mut response = client.get("/x").dispatch();
    assert_eq!(response.body_string(), Some("This is x route".into()));

    let mut response = client.get("/test/y").dispatch();
    assert_eq!(response.body_string(), Some("This is y route in test module".into()));

    let mut response = client.get("/test/z").dispatch();
    assert_eq!(response.body_string(), Some("This is z route in test module".into()));

    let response = client.get("/y").dispatch();
    assert_eq!(response.status(), Status::NotFound);

    let response = client.get("/z").dispatch();
    assert_eq!(response.status(), Status::NotFound);

    let response = client.get("/w").dispatch();
    assert_eq!(response.status(), Status::NotFound);

    let response = client.get("/test/x").dispatch();
    assert_eq!(response.status(), Status::NotFound);
}
