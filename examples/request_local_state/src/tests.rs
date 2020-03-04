use std::sync::atomic::{Ordering};

use super::{rocket, Atomics};
use rocket::local::Client;

#[rocket::async_test]
async fn test() {
    let client = Client::new(rocket()).unwrap();
    client.get("/sync").dispatch().await;

    let atomics = client.manifest().state::<Atomics>().unwrap();
    assert_eq!(atomics.uncached.load(Ordering::Relaxed), 2);
    assert_eq!(atomics.cached.load(Ordering::Relaxed), 1);

    client.get("/async").dispatch().await;

    let atomics = client.manifest().state::<Atomics>().unwrap();
    assert_eq!(atomics.uncached.load(Ordering::Relaxed), 4);
    assert_eq!(atomics.cached.load(Ordering::Relaxed), 2);
}
