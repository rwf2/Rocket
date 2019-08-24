use std::sync::atomic::{Ordering};

use super::{rocket, Atomics};
use rocket::local::Client;

#[rocket::async_test]
async fn test() {
    let client = Client::new(rocket()).unwrap();
    client.get("/").dispatch().await;

    let atomics = client.rocket().state::<Atomics>().unwrap();
    assert_eq!(atomics.uncached.load(Ordering::Relaxed), 2);
    assert_eq!(atomics.cached.load(Ordering::Relaxed), 1);
}
