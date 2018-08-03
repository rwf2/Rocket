#![feature(plugin, decl_macro, proc_macro_non_items)]
#![plugin(rocket_codegen)]

use rocket;
use rocket::routes;

#[cfg(test)] mod tests;

#[get("/")]
fn hello() -> &'static str {
    "Hello, Rust 2018!"
}

fn main() {
    rocket::ignite().mount("/", routes![hello]).launch();
}
