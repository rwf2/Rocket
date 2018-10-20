#![feature(proc_macro_hygiene, decl_macro)]

use rocket::{get, routes, uri};

#[cfg(test)] mod tests;

#[get("/")]
fn hello() -> String {
    format!("Hello! Try {}.", uri!(hello_name: "Rust 2018"))
}

#[rocket::get("/<name>")]
fn hello_name(name: String) -> String {
    format!("Hello, {}!", name)
}

fn main() {
    rocket::ignite().mount("/", routes![hello]).launch();
}
