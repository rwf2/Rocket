#![feature(proc_macro_hygiene)]

#[macro_use] extern crate rocket;

fn main() {
    let _ = catchers![a b]; //~ ERROR expected
    let _ = catchers![];
    let _ = catchers![a::, ]; //~ ERROR expected identifier
    let _ = catchers![a::]; //~ ERROR expected identifier
}
