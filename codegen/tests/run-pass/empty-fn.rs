#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

#[get("")]
fn get() -> &'static str { "hi" }

#[get("/")]
fn get_empty() {  }

fn main() { }
