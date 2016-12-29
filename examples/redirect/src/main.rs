#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

#[cfg(test)] mod tests;

use rocket::response::Redirect;

#[get("/")]
fn root() -> Redirect {
    Redirect::to("/login")
}

#[get("/login")]
fn login() -> &'static str {
    "Hi! Please log in before continuing."
}

fn main() {
    rocket::ignite().mount("/", routes![root, login]).launch();
}
