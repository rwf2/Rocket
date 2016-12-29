#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate redirect;

fn main() {
    rocket::ignite().mount("/", routes![redirect::root, redirect::login]).launch();
}
