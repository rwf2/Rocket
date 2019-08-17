#![feature(proc_macro_hygiene)]

#[macro_use] extern crate rocket;

extern crate rocket_contrib;

#[post("/")]
fn hello() -> &'static str {
    "Hello, world!"
}


fn main() {
    rocket::ignite()
        .mount("/", routes![hello])
        .attach(rocket_contrib::cors::CorsFairing::new())
        .launch();
}