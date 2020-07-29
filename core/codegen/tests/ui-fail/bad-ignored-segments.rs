#[macro_use] extern crate rocket;

// Check for ignores in invalid spaces

#[get("/c?<_>")]
fn i1() {}

#[post("/d", data = "<_>")]
fn i2() {}

fn main() {  }
