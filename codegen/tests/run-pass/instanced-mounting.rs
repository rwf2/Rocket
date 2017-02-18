#![feature(plugin)]
#![plugin(rocket_codegen)]
#![allow(dead_code)]
#![deny(unmounted_route)]

extern crate rocket;

use rocket::State;

#[get("/one")]
fn one() {  }

#[get("/two")]
fn two() {  }

#[get("/three")]
fn three() {  }

#[get("/four")]
fn four() {  }

fn main() {
    let instance = rocket::ignite()
        .mount("/", routes![one]);

    let other = instance.mount("/", routes![two]);
    other.mount("/", routes![three])
        .mount("/", routes![four]);

    rocket::ignite()
        .mount("/", routes![one])
        .mount("/", routes![two])
        .mount("/", routes![three])
        .mount("/", routes![four]);

    let a = rocket::ignite()
        .mount("/", routes![one])
        .mount("/", routes![two]);

    let b = a.mount("/", routes![three])
        .mount("/", routes![four]);
}
