#![feature(proc_macro_hygiene)]

#[macro_use] extern crate rocket;

#[cfg(test)] mod tests;

use std::{io, env};
use rocket::{Data, response::Debug};

#[post("/upload", format = "plain", data = "<data>")]
async fn upload(data: Data) -> Result<String, Debug<io::Error>> {
    Ok(data.stream_to_file(env::temp_dir().join("upload.txt")).await?.to_string())
}

#[get("/")]
fn index() -> &'static str {
    "Upload your text files by POSTing them to /upload."
}

fn rocket() -> rocket::Rocket {
    rocket::ignite().mount("/", routes![index, upload])
}

#[rocket::main]
async fn main() {
    let _ = rocket().launch().await;
}
