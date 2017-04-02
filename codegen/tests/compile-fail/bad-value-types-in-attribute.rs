#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

#[get(1)]  //~ ERROR expected `path = string`
fn get0() -> &'static str { "hi" }

#[get(path = 1)]  //~ ERROR must be a string
fn get1() -> &'static str { "hi" }

#[get(path = "/", rank = "2")]  //~ ERROR must be an int
fn get2() -> &'static str { "hi" }

#[get(path = "/", format = 100)]  //~ ERROR must be a "media/type"
fn get3() -> &'static str { "hi" }

fn main() {
}

