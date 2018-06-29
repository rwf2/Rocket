use std::sync::atomic::{Ordering};

use ::Atomics;
use super::rocket;
use rocket::local::Client;

#[test]
fn test() {
    let client = Client::new(rocket()).unwrap();
    client.get("/").dispatch();

    let atomics = &client.rocket().state::<Atomics>().unwrap();
    assert!(atomics.uncached.load(Ordering::Relaxed) == 2, "First is two");
    assert!(atomics.cached.load(Ordering::Relaxed) == 1, "Second is one");
}
