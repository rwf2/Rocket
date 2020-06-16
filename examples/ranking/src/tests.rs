use rocket::local::Client;

async fn test(uri: String, expected: String) {
    let client = Client::new(super::rocket()).await.unwrap();
    let mut response = client.get(&uri).dispatch().await;
    assert_eq!(response.body_string().await, Some(expected));
}

#[rocket::async_test]
async fn test_hello() {
    for &(name, age) in &[("Mike", 22), ("Michael", 80), ("A", 0), ("a", 127)] {
        test(format!("/hello/{}/{}", name, age),
            format!("Hello, {} year old named {}!", age, name)).await;
    }
}

#[rocket::async_test]
async fn test_failing_hello_hi() {
    // Invalid integers.
    for &(name, age) in &[("Mike", 1000), ("Michael", 128), ("A", -800), ("a", -200)] {
        test(format!("/hello/{}/{}", name, age),
            format!("Hi {}! Your age ({}) is kind of funky.", name, age)).await;
    }

    // Non-integers.
    for &(name, age) in &[("Mike", "!"), ("Michael", "hi"), ("A", "blah"), ("a", "0-1")] {
        test(format!("/hello/{}/{}", name, age),
            format!("Hi {}! Your age ({}) is kind of funky.", name, age)).await;
    }
}
