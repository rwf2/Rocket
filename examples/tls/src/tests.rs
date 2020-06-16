use rocket::local::Client;

#[rocket::async_test]
async fn hello_world() {
    let client = Client::new(super::rocket()).await.unwrap();
    let mut response = client.get("/").dispatch().await;
    assert_eq!(response.body_string().await, Some("Hello, world!".into()));
}
