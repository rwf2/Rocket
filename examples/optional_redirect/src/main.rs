#![feature(plugin)]
#![plugin(rocket_codegen)]
extern crate rocket;

#[cfg(test)]
mod tests;

use rocket::response::Redirect;

#[get("/")]
fn root() -> Redirect {
    Redirect::to("/users/login")
}

#[get("/users/<name>")]
fn user(name: &str) -> Result<&'static str, Redirect> {
    match name {
        "Sergio" => Ok("Hello, Sergio!"),
        _ => Err(Redirect::to("/users/login")),
    }
}

#[get("/users/login")]
fn login() -> &'static str {
    "Hi! That user doesn't exist. Maybe you need to log in?"
}

fn main() {
    rocket::ignite().mount("/", routes![root, user, login]).launch()
}
