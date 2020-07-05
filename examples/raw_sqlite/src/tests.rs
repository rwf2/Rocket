use super::rocket;
use rocket::local::blocking::Client;

#[test]
fn hello() {
    let client = Client::new(rocket()).unwrap();
    let response = client.get("/").dispatch();
    assert_eq!(response.into_string(), Some("Rocketeer".into()));
}
