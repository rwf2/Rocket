#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

use std::io;

use rocket::response::NamedFile;

#[cfg(test)] mod tests;

#[get("/")]
fn index() -> io::Result<NamedFile> {
    NamedFile::open("static/index.html")
}

#[put("/")]
fn put() -> &'static str {
    "Hello, PUT request!"
}

fn main() {
    rocket::ignite()
        .mount("/", routes![index, put])
        .launch();
}
