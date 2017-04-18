#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

use rocket::response::{content, Stream};

use std::io::{self, repeat, Repeat, Read, Take};
use std::fs::File;

type LimitedRepeat = Take<Repeat>;

#[get("/")]
fn root() -> content::Plain<Stream<LimitedRepeat>> {
    content::Plain(Stream::from(repeat('a' as u8).take(25000)))
}

#[get("/big_file")]
fn file() -> io::Result<Stream<File>> {
    // Generate this file using: head -c BYTES /dev/random > big_file.dat
    const FILENAME: &'static str = "big_file.dat";
    File::open(FILENAME).map(|file| Stream::from(file))
}

fn main() {
    rocket::ignite().mount("/", routes![root, file]).launch();
}
