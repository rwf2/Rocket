#![feature(proc_macro_hygiene)]

#[macro_use] extern crate rocket;

fn main() {
    let _ = routes![a b]; //~ ERROR expected `,`
    let _ = routes![];
    let _ = routes![a::, ]; //~ ERROR expected identifier
    let _ = routes![a::]; //~ ERROR expected identifier
}
