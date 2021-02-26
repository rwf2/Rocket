use rocket::local::blocking::Client;

struct SpawnBlockingOnDrop;

impl Drop for SpawnBlockingOnDrop {
    fn drop(&mut self) {
        rocket::tokio::task::spawn_blocking(|| ());
    }
}

#[test]
fn test_access_runtime_in_state_drop() {
    Client::tracked(rocket::ignite().manage(SpawnBlockingOnDrop)).unwrap();
}
