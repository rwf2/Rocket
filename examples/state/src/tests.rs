use rocket::local::asynchronous::Client;
use rocket::http::Status;

async fn register_hit(client: &Client) {
    let response = client.get("/").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
}

async fn get_count(client: &Client) -> usize {
    let response = client.get("/count").dispatch().await;
    response.into_string().await.and_then(|s| s.parse().ok()).unwrap()
}

#[rocket::async_test]
async fn test_count() {
    let client = Client::new(super::rocket()).await.unwrap();

    // Count should start at 0.
    assert_eq!(get_count(&client).await, 0);

    for _ in 0..99 { register_hit(&client).await; }
    assert_eq!(get_count(&client).await, 99);

    register_hit(&client).await;
    assert_eq!(get_count(&client).await, 100);
}

#[rocket::async_test]
async fn test_raw_state_count() {
    use rocket::State;
    use super::{count, index};

    let mut rocket = super::rocket();
    let cargo = rocket.inspect().await;

    assert_eq!(count(State::from(cargo).unwrap()), "0");
    assert!(index(State::from(cargo).unwrap()).0.contains("Visits: 1"));
    assert_eq!(count(State::from(cargo).unwrap()), "1");
}

// Cargo runs each test in parallel on different threads. We use all of these
// tests below to show (and assert) that state is managed per-Rocket instance.
#[test] fn test_count_parallel() { test_count() }
#[test] fn test_count_parallel_2() { test_count() }
#[test] fn test_count_parallel_3() { test_count() }
#[test] fn test_count_parallel_4() { test_count() }
#[test] fn test_count_parallel_5() { test_count() }
#[test] fn test_count_parallel_6() { test_count() }
#[test] fn test_count_parallel_7() { test_count() }
#[test] fn test_count_parallel_8() { test_count() }
#[test] fn test_count_parallel_9() { test_count() }
