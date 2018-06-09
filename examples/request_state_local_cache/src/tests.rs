use std::time::{Instant, Duration};

use super::rocket;
use rocket::local::Client;

fn normal_time() -> Duration {
    let client = Client::new(rocket()).unwrap();

    let start = Instant::now();
    client.get("/").dispatch();
    let end = Instant::now();

    end - start
}

fn cached_time() -> Duration {
    let client = Client::new(rocket()).unwrap();

    let start = Instant::now();
    client.get("/cached").dispatch();
    let end = Instant::now();

    end - start
}

#[test]
fn cache_is_faster() {
    let normal = normal_time();
    let cached = cached_time();

    assert!(normal > cached, "Cached is faster");
    assert!((normal - cached).as_secs() >= 3, "Cached is at least 3 secs faster");
}
