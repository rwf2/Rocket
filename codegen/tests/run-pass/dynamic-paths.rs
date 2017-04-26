#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

#[get("/test/<one>/<two>/<three>")]
fn get(one: String, two: usize, three: isize) -> &'static str { "hi" }

fn main() {
    let _ = routes![get];
}
