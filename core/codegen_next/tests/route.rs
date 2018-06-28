#![feature(plugin, decl_macro, proc_macro_non_items)]
#![plugin(rocket_codegen)]

extern crate rocket;

use rocket::routes;

#[get("/")]
fn get() {}

fn main() {
    rocket::ignite().mount("/", routes![get]);
}
