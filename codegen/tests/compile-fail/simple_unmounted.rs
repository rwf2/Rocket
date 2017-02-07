#![feature(plugin)]
#![plugin(rocket_codegen)]
#![allow(dead_code)]
#![deny(unmounted_route)]

extern crate rocket;

#[get("/")]
fn index() {  }
//~^ ERROR is not mounted

fn main() {
    rocket::ignite().launch();
}
