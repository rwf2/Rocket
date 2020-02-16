use super::rocket;
use rocket::local::Client;

#[rocket::async_test]
async fn hello() {
    let client = Client::new(rocket()).await.unwrap();
    let mut response = client.get("/").dispatch().await;
    assert_eq!(response.body_string().await, Some("Rocketeer".into()));
}
