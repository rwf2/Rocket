#![feature(plugin, custom_derive)]
#![plugin(rocket_codegen)]

extern crate rocket;

#[post("/", format = "application/x-custom")]
fn get() -> &'static str { "hi" }

fn main() { }
