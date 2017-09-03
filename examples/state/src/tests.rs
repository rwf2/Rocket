use rocket::local::Client;
use rocket::http::Status;

fn register_hit(client: &Client) {
    let response = client.get("/").dispatch();;
    assert_eq!(response.status(), Status::Ok);
}

fn get_count(client: &Client) -> usize {
    let mut response = client.get("/count").dispatch();
    response.body_string().and_then(|s| s.parse().ok()).unwrap()
}

#[test]
fn test_count() {
    let client = Client::new(super::rocket()).unwrap();

    // Count should start at 0.
    assert_eq!(get_count(&client), 0);

    for _ in 0..99 { register_hit(&client); }
    assert_eq!(get_count(&client), 99);

    register_hit(&client);
    assert_eq!(get_count(&client), 100);
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
