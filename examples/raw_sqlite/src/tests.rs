use super::rocket;
use rocket::local::Client;

#[test]
fn hello() {
    let client = Client::new(rocket()).unwrap();
    let mut response = client.get("/").dispatch();
    assert_eq!(response.body_string(), Some("Rocketeer".into()));
}
