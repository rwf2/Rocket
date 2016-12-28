#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

#[get("/")]
pub fn hello() -> &'static str {
    "Hello, world!"
}
