#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

#[get(path = "/hello", 123)]  //~ ERROR expected
fn get() -> &'static str { "hi" }

fn main() {
    let _ = routes![get];
}

