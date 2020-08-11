#[macro_use] extern crate rocket;

fn main() {
    let _ = catchers![a b];
    let _ = catchers![];
    let _ = catchers![a::, ];
    let _ = catchers![a::];
}
