#[macro_use] extern crate rocket;

#[cfg(test)] mod tests;

use rocket::response::{content, Stream};

use tokio::fs::File;
use tokio::io::{repeat, AsyncRead, AsyncReadExt};

// Generate this file using: head -c BYTES /dev/random > big_file.dat
const FILENAME: &str = "big_file.dat";

#[get("/")]
fn root() -> content::Plain<Stream<impl AsyncRead>> {
    content::Plain(Stream::from(repeat('a' as u8).take(25000)))
}

#[get("/big_file")]
async fn file() -> Option<Stream<File>> {
    File::open(FILENAME).await.map(Stream::from).ok()
}

#[launch]
fn rocket() -> rocket::Rocket {
    rocket::ignite().mount("/", routes![root, file])
}
