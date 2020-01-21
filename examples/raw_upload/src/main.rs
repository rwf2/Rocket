#![feature(proc_macro_hygiene)]

#[macro_use]
extern crate rocket;

#[cfg(test)]
mod tests;

use rocket::{response::Debug, Data};
use std::{env, io};

#[post("/upload", format = "plain", data = "<data>")]
fn upload(data: Data) -> Result<String, Debug<io::Error>> {
    data.stream_to_file(env::temp_dir().join("upload.txt"))
        .map(|n| n.to_string())
        .map_err(Debug)
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
