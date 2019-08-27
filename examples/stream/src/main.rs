#![feature(proc_macro_hygiene)]

#[macro_use] extern crate rocket;

#[cfg(test)] mod tests;

use rocket::response::{content, Stream};

use std::io::{self, repeat};

use tokio::fs::File;
use futures_tokio_compat::Compat as TokioCompat;

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
async fn file() -> io::Result<Stream<TokioCompat<File>>> {
    File::open(FILENAME).await.map(|file| Stream::from(TokioCompat::new(file)))
}

fn rocket() -> rocket::Rocket {
    rocket::ignite().mount("/", routes![root, file])
}

fn main() {
    rocket().launch();
}
