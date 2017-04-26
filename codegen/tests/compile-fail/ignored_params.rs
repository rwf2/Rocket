#![feature(plugin)]
#![plugin(rocket_codegen)]

#[get("/<name>")] //~ ERROR 'name' is declared
fn get(other: usize) -> &'static str { "hi" } //~ ERROR isn't in the function

#[get("/a?<r>")] //~ ERROR 'r' is declared
fn get1() -> &'static str { "hi" } //~ ERROR isn't in the function

#[post("/a", data = "<test>")] //~ ERROR 'test' is declared
fn post() -> &'static str { "hi" } //~ ERROR isn't in the function

fn main() {  }
