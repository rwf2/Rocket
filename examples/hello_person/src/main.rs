#![feature(plugin, decl_macro, proc_macro_hygiene)]
#![plugin(rocket_codegen)]

#[macro_use] extern crate rocket;

#[cfg(test)] mod tests;

#[get("/hello/<name>/<age>")]
fn hello(name: String, age: u8) -> String {
    format!("Hello, {} year old named {}!", age, name)
}

#[get("/hello/<name>")]
fn hi(name: String) -> String {
    name
}

fn main() {
    rocket::ignite().mount("/", routes![hello, hi]).launch();
}
