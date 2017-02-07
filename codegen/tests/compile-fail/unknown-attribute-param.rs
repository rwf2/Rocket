#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

#[get(path = "/hello", unknown = 123)]  //~ ERROR 'unknown' is not a known param
fn get() -> &'static str { "hi" }

fn main() {
    let _ = routes![get];
}

