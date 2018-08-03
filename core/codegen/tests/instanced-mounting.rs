#![feature(plugin, decl_macro, proc_macro_non_items)]
#![plugin(rocket_codegen)]
#![allow(dead_code, unused_variables)]

#[macro_use] extern crate rocket;

#[get("/one")]
fn one() {  }

#[get("/two")]
fn two() {  }

#[get("/three")]
fn three() {  }

#[get("/four")]
fn four() {  }

#[test]
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
