use super::rocket;
use rocket::local::asynchronous::Client;

#[rocket::async_test]
async fn hello() {
    let client = Client::new(rocket()).await.unwrap();
    let response = client.get("/").dispatch().await;
    assert_eq!(response.into_string().await, Some("Rocketeer".into()));
}
