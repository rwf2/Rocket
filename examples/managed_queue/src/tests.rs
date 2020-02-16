use rocket::local::Client;
use rocket::http::Status;

#[rocket::async_test]
async fn test_push_pop() {
    let client = Client::new(super::rocket()).await.unwrap();

    let response = client.put("/push?event=test1").dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let mut response = client.get("/pop").dispatch().await;
    assert_eq!(response.body_string().await, Some("test1".to_string()));
}
