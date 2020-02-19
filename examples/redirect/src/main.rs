#![feature(proc_macro_hygiene)]

#[macro_use] extern crate rocket;

#[cfg(test)] mod tests;

use rocket::response::Redirect;

#[get("/")]
fn root() -> Redirect {
    Redirect::to(uri!(login))
}

#[get("/login")]
fn login() -> &'static str {
    "Hi! Please log in before continuing."
}

#[rocket::main]
async fn main() {
    let _ = rocket::ignite().mount("/", routes![root, login]).launch().await;
}
