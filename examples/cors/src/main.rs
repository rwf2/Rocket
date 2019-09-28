#![feature(proc_macro_hygiene)]

#[macro_use] extern crate rocket;

extern crate rocket_contrib;

use rocket::Data;
use rocket_contrib::cors::CorsFairing;

#[get("/test")]
fn get_index() -> &'static str {
    "Hi"
}

#[post("/")]
fn hello() -> &'static str {
    "Hello, world!"
}

#[patch("/notes/<_id>", data="<_input>")]
fn patch_notes(_id: u64, _input: Data) -> &'static str {
    "Hello, world!"
}


fn main() {
    rocket::ignite()
        .mount("/", routes![hello, get_index, patch_notes])
        .attach(CorsFairing::new())
        .launch();
}
