#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate rocket_contrib;

use rocket::http::Method;
use rocket_contrib::{PreflightCORS, CORS};

#[cfg(test)]
mod tests;

#[get("/hello")]
fn hello() -> CORS<String> {
    CORS::any("Hello there!".to_string())
}

fn main() {
    rocket::ignite()
        .mount("/", routes![cors_preflight, hello])
        .launch();
}
