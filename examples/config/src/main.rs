#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

// This example's illustration is the Rocket.toml file.
fn main() {
    rocket::ignite().launch();
}
