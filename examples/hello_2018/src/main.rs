#![feature(plugin, decl_macro, proc_macro_hygiene)]
#![plugin(rocket_codegen)]

use rocket::{self, routes};

#[cfg(test)] mod tests;

#[get("/")]
fn hello() -> &'static str {
    "Hello, Rust 2018!"
}

fn main() {
    rocket::ignite().mount("/", routes![hello]).launch();
}
