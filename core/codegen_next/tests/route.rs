#![feature(plugin, decl_macro, proc_macro_non_items)]
#![plugin(rocket_codegen)]

#[macro_use] extern crate rocket;

#[get("/")]
fn get() {}

fn main() {
    rocket::ignite().mount("/", routes![get]);
}
