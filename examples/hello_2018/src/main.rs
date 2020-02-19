#![feature(proc_macro_hygiene)]

#[cfg(test)] mod tests;

use rocket::{get, routes};

#[get("/")]
fn hello() -> &'static str {
    "Hello, Rust 2018!"
}

#[rocket::main]
async fn main() {
    let _ = rocket::ignite().mount("/", routes![hello]).launch().await;
}
