use rocket::local::asynchronous::Client;

#[rocket::async_test]
async fn hello_world() {
    let client = Client::new(super::rocket()).await.unwrap();
    let response = client.get("/").dispatch().await;
    assert_eq!(response.into_string().await, Some("Hello, world!".into()));
}
