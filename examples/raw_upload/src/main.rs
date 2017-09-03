#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

#[cfg(test)] mod tests;

use std::io;
use rocket::Data;

#[post("/upload", format = "text/plain", data = "<data>")]
fn upload(data: Data) -> io::Result<String> {
    data.stream_to_file("/tmp/upload.txt").map(|n| n.to_string())
}

#[get("/")]
fn index() -> &'static str {
    "Upload your text files by POSTing them to /upload."
}

fn rocket() -> rocket::Rocket {
    rocket::ignite().mount("/", routes![index, upload])
}

fn main() {
    rocket().launch();
}
