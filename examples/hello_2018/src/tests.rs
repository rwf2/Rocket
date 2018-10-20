use rocket::{self, routes, local::Client, Rocket};

fn rocket() -> Rocket {
    rocket::ignite().mount("/", routes![super::hello, super::hello_name])
}

#[test]
fn hello() {
    let client = Client::new(rocket()).unwrap();
    let mut response = client.get("/").dispatch();
    assert_eq!(response.body_string(), Some("Hello! Try /Rust%202018.".into()));
}

#[test]
fn hello_name() {
    let client = Client::new(rocket()).unwrap();
    let mut response = client.get("/Rust%202018").dispatch();
    assert_eq!(response.body_string(), Some("Hello, Rust 2018!".into()));
}
