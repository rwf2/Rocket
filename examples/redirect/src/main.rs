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

#[launch]
fn rocket() -> rocket::Rocket {
    rocket::ignite().mount("/", routes![root, login])
}
