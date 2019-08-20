#![feature(proc_macro_hygiene)]

#[cfg(test)] mod tests;

use rocket::{get, routes};

#[get("/")]
fn hello() -> &'static str {
    "Hello, Rust 2018!"
}

fn main() {
    rocket::ignite().mount("/", routes![hello]).launch();
}
