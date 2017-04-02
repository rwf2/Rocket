#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

#[cfg(test)] mod tests;

use rocket::http::RawStr;

#[get("/users/<name>")]
fn user(name: &RawStr) -> Option<&'static str> {
    if name == "Sergio" {
        Some("Hello, Sergio!")
    } else {
        None
    }
}

fn main() {
    rocket::ignite().mount("/", routes![user]).launch();
}
