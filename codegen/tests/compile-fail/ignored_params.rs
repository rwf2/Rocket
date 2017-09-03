#![feature(plugin)]
#![plugin(rocket_codegen)]

#[get("/<name>")] //~ ERROR unused dynamic parameter: `name`
fn get(other: usize) -> &'static str { "hi" } //~ NOTE expected

#[get("/a?<r>")] //~ ERROR unused dynamic parameter: `r`
fn get1() -> &'static str { "hi" } //~ NOTE expected

#[post("/a", data = "<test>")] //~ ERROR unused dynamic parameter: `test`
fn post() -> &'static str { "hi" } //~ NOTE expected

#[get("/<_r>")] //~ ERROR unused dynamic parameter: `_r`
fn get2(r: usize) -> &'static str { "hi" } //~ NOTE expected

fn main() {  }
