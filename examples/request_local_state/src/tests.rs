use std::sync::atomic::Ordering;

use super::{rocket, Atomics};
use rocket::local::blocking::Client;

#[test]
fn test() {
    let client = Client::tracked(rocket()).unwrap();
    client.get("/sync").dispatch();

    let atomics = client.rocket().state::<Atomics>().unwrap();
    assert_eq!(atomics.uncached.load(Ordering::Relaxed), 2);
    assert_eq!(atomics.cached.load(Ordering::Relaxed), 1);

    client.get("/async").dispatch();

    let atomics = client.rocket().state::<Atomics>().unwrap();
    assert_eq!(atomics.uncached.load(Ordering::Relaxed), 4);
    assert_eq!(atomics.cached.load(Ordering::Relaxed), 2);
}
