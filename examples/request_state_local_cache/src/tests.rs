use std::sync::atomic::{Ordering};

use ::Atomics;
use super::rocket;
use rocket::local::Client;

#[test]
fn test() {
    let client = Client::new(rocket()).unwrap();
    client.get("/").dispatch();

    assert!(client.rocket().state::<Atomics>().unwrap().first.load(Ordering::Relaxed) == 2, "First is two");
    assert!(client.rocket().state::<Atomics>().unwrap().second.load(Ordering::Relaxed) == 1, "Secon is one");
}
