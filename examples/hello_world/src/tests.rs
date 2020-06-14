use rocket::local::Client;

#[rocket::async_test]
async fn hello_world() {
    let rocket = rocket::ignite().mount("/", routes![super::hello]);
    let client = Client::new(rocket).await.unwrap();
    let mut response = client.get("/").dispatch().await;
    assert_eq!(response.body_string().await, Some("Hello, world!".into()));
}
