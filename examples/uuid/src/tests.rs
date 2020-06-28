use super::rocket;
use rocket::local::asynchronous::Client;
use rocket::http::Status;

async fn test(uri: &str, expected: &str) {
    let client = Client::new(rocket()).await.unwrap();
    let res = client.get(uri).dispatch().await;
    assert_eq!(res.into_string().await, Some(expected.into()));
}

async fn test_404(uri: &str) {
    let client = Client::new(rocket()).await.unwrap();
    let res = client.get(uri).dispatch().await;
    assert_eq!(res.status(), Status::NotFound);
}

#[rocket::async_test]
async fn test_people() {
    test("/people/7f205202-7ba1-4c39-b2fc-3e630722bf9f", "We found: Lacy").await;
    test("/people/4da34121-bc7d-4fc1-aee6-bf8de0795333", "We found: Bob").await;
    test("/people/ad962969-4e3d-4de7-ac4a-2d86d6d10839", "We found: George").await;
    test("/people/e18b3a5c-488f-4159-a240-2101e0da19fd",
         "Person not found for UUID: e18b3a5c-488f-4159-a240-2101e0da19fd").await;
    test_404("/people/invalid_uuid").await;
}
