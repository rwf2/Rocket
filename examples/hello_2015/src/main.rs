#![feature(proc_macro_hygiene, async_await)]

#[macro_use] extern crate rocket;

#[cfg(test)] mod tests;

#[get("/")]
fn hello() -> &'static str {
    "Hello, Rust 2015!"
}

fn main() {
    rocket::ignite().mount("/", routes![hello]).launch();
}
