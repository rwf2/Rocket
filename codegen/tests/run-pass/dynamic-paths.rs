#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

#[get("/test/<one>/<two>/<three>")]
fn get(one: String, two: usize, three: isize) -> &'static str { "hi" }

#[get("/test/<_one>/<_two>/<__three>")]
fn ignored(_one: String, _two: usize, __three: isize) -> &'static str { "hi" }

fn main() {
    let _ = routes![get, ignored];
}
