#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

use rocket::response::{content, Stream};

use std::io::{self, repeat, Repeat, Read, Take};
use std::fs::File;

type LimitedRepeat = Take<Repeat>;

#[get("/")]
fn root() -> content::Plain<Stream<LimitedRepeat>> {
    content::Plain(repeat('a' as u8).take(25000).into())
}

#[get("/big_file")]
fn file() -> io::Result<Stream<File>> {
    // Generate this file using: head -c BYTES /dev/random > big_file.dat
    const FILENAME: &'static str = "big_file.dat";
    File::open(FILENAME).map(|file| file.into())
}

fn main() {
    rocket::ignite().mount("/", routes![root, file]).launch();
}
