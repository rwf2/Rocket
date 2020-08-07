#[macro_use] extern crate rocket;

fn main() {
    let _ = routes![a b];
    let _ = routes![];
    let _ = routes![a::, ];
    let _ = routes![a::];
}
