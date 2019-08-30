#![feature(proc_macro_hygiene)]

#[macro_use] extern crate rocket;

#[cfg(test)] mod tests;

use rocket::response::{content, Stream};

use std::io::{self, repeat};
use async_std::fs::File;

use rocket::AsyncReadExt as _;

//type LimitedRepeat = Take<Repeat>;
type LimitedRepeat = Box<dyn futures::io::AsyncRead + Send + Unpin>;

// Generate this file using: head -c BYTES /dev/random > big_file.dat
const FILENAME: &str = "big_file.dat";

#[get("/")]
fn root() -> content::Plain<Stream<LimitedRepeat>> {
    content::Plain(Stream::from(Box::new(repeat('a' as u8).take(25000)) as Box<_>))
}

#[get("/big_file")]
async fn file() -> io::Result<Stream<File>> {
    File::open(FILENAME).await.map(Stream::from)
}

fn rocket() -> rocket::Rocket {
    rocket::ignite().mount("/", routes![root, file])
}

fn main() {
    let _ = rocket().launch();
}
