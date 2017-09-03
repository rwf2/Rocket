#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

#[get("/")]
fn get(_: usize) -> &'static str { "hi" } //~ ERROR argument

fn main() {  }
