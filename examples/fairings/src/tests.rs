use super::rocket;
use rocket::local::blocking::Client;

#[test]
fn rewrite_get_put() {
    let client = Client::tracked(rocket()).unwrap();
    let response = client.get("/").dispatch();
    assert_eq!(response.into_string(), Some("Hello, fairings!".into()));
}

#[test]
fn counts() {
    let client = Client::tracked(rocket()).unwrap();

    // Issue 1 GET request.
    client.get("/").dispatch();

    // Check the GET count, taking into account _this_ GET request.
    let response = client.get("/counts").dispatch();
    assert_eq!(response.into_string(), Some("Get: 2\nPost: 0".into()));

    // Issue 1 more GET request and a POST.
    client.get("/").dispatch();
    client.post("/").dispatch();

    // Check the counts.
    let response = client.get("/counts").dispatch();
    assert_eq!(response.into_string(), Some("Get: 4\nPost: 1".into()));
}

#[test]
fn token() {
    let client = Client::tracked(rocket()).unwrap();

    // Ensure the token is '123', which is what we have in `Rocket.toml`.
    let res = client.get("/token").dispatch();
    assert_eq!(res.into_string(), Some("123".into()));
}
