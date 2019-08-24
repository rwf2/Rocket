use super::rocket;
use rocket::local::Client;

#[rocket::async_test]
async fn rewrite_get_put() {
    let client = Client::new(rocket()).unwrap();
    let mut response = client.get("/").dispatch().await;
    assert_eq!(response.body_string().await, Some("Hello, fairings!".into()));
}

#[rocket::async_test]
async fn counts() {
    let client = Client::new(rocket()).unwrap();

    // Issue 1 GET request.
    client.get("/").dispatch().await;

    // Check the GET count, taking into account _this_ GET request.
    let mut response = client.get("/counts").dispatch().await;
    assert_eq!(response.body_string().await, Some("Get: 2\nPost: 0".into()));

    // Issue 1 more GET request and a POST.
    client.get("/").dispatch().await;
    client.post("/").dispatch().await;

    // Check the counts.
    let mut response = client.get("/counts").dispatch().await;
    assert_eq!(response.body_string().await, Some("Get: 4\nPost: 1".into()));
}

#[rocket::async_test]
async fn token() {
    let client = Client::new(rocket()).unwrap();

    // Ensure the token is '123', which is what we have in `Rocket.toml`.
    let mut res = client.get("/token").dispatch().await;
    assert_eq!(res.body_string().await, Some("123".into()));
}
