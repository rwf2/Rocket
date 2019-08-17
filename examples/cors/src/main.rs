#![feature(proc_macro_hygiene)]

#[macro_use] extern crate rocket;

extern crate rocket_contrib;

#[get("/test")]
fn get_index() -> &'static str {
    "Hi"
}

#[post("/")]
fn hello() -> &'static str {
    "Hello, world!"
}


fn main() {
    rocket::ignite()
        .mount("/", routes![hello, get_index])
        .attach(rocket_contrib::cors::CorsFairing::new())
        .launch();
}
