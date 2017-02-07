#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

#[route(CONNECT, "/hello")]  //~ ERROR valid HTTP method
fn get() -> &'static str { "hi" }

fn main() {
    let _ = routes![get];
}

