#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

#[route(FIX, "/hello")]  //~ ERROR is not a valid HTTP method
//~^ ERROR valid HTTP method
fn get() -> &'static str { "hi" }

fn main() {
    let _ = routes![get];
}

