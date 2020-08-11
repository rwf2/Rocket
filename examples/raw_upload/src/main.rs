#[macro_use] extern crate rocket;

#[cfg(test)] mod tests;

use std::{io, env};
use rocket::data::{Data, ToByteUnit};
use rocket::response::Debug;

#[post("/upload", format = "plain", data = "<data>")]
async fn upload(data: Data) -> Result<String, Debug<io::Error>> {
    let path = env::temp_dir().join("upload.txt");
    Ok(data.open(128.kibibytes()).stream_to_file(path).await?.to_string())
}

#[get("/")]
fn index() -> &'static str {
    "Upload your text files by POSTing them to /upload."
}

#[launch]
fn rocket() -> rocket::Rocket {
    rocket::ignite().mount("/", routes![index, upload])
}
