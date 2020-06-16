use rocket::local::Client;
use rocket::http::Status;

async fn test(uri: String, expected: String) {
    let client = Client::new(super::rocket()).await.unwrap();
    assert_eq!(client.get(&uri).dispatch().await.body_string().await, Some(expected));
}

async fn test_404(uri: &'static str) {
    let client = Client::new(super::rocket()).await.unwrap();
    assert_eq!(client.get(uri).dispatch().await.status(), Status::NotFound);
}

#[rocket::async_test]
async fn test_hello() {
    for &(name, age) in &[("Mike", 22), ("Michael", 80), ("A", 0), ("a", 127)] {
        test(format!("/hello/{}/{}", name, age),
            format!("Hello, {} year old named {}!", age, name)).await;
    }
}

#[rocket::async_test]
async fn test_failing_hello() {
    test_404("/hello/Mike/1000").await;
    test_404("/hello/Mike/-129").await;
    test_404("/hello/Mike/-1").await;
}

#[rocket::async_test]
async fn test_hi() {
    for name in &["Mike", "A", "123", "hi", "c"] {
        test(format!("/hello/{}", name), name.to_string()).await;
    }
}
