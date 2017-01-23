use rocket::Rocket;
use rocket::testing::MockRequest;
use rocket::http::Method::*;
use rocket::http::Status;

fn register_hit(rocket: &Rocket) {
    let mut req = MockRequest::new(Get, "/");
    let response = req.dispatch_with(&rocket);
    assert_eq!(response.status(), Status::Ok);
}

fn get_count(rocket: &Rocket) -> usize {
    let mut req = MockRequest::new(Get, "/count");
    let mut response = req.dispatch_with(&rocket);
    let body_string = response.body().and_then(|b| b.into_string()).unwrap();
    body_string.parse().unwrap()
}

#[test]
fn test_count() {
    let rocket = super::rocket();

    // Count should start at 0.
    assert_eq!(get_count(&rocket), 0);

    for _ in 0..99 { register_hit(&rocket); }
    assert_eq!(get_count(&rocket), 99);

    register_hit(&rocket);
    assert_eq!(get_count(&rocket), 100);
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
