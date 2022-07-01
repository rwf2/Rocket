use rocket::{Rocket, Build};
use rocket::local::blocking::Client;

mod inner {
    use rocket::uri;

    #[rocket::get("/")]
    pub fn hello() -> String {
        format!("Hello! Try {}.", uri!(super::hello_name("Rust 2018")))
    }
}

#[rocket::get("/<name>")]
fn hello_name(name: String) -> String {
    format!("Hello, {}! This is {}.", name, rocket::uri!(hello_name(&name)))
}

fn rocket() -> Rocket<Build> {
    rocket::build()
        .mount("/", rocket::routes![hello_name])
        .mount("/", rocket::routes![inner::hello])
}

#[test]
fn test_inner_hello() {
    let client = Client::debug(rocket()).unwrap();
    let response = client.get("/").dispatch();
    assert_eq!(response.into_string(), Some("Hello! Try /Rust%202018.".into()));
}

#[test]
fn test_hello_name() {
    let client = Client::debug(rocket()).unwrap();
    let response = client.get("/Rust%202018").dispatch();
    assert_eq!(response.into_string().unwrap(), "Hello, Rust 2018! This is /Rust%202018.");
}
