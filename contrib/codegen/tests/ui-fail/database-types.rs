extern crate rocket;
#[macro_use] extern crate rocket_contrib;

struct Unknown;

#[database("foo")]
struct A(Unknown);

#[database("foo")]
struct B(Vec<i32>);

fn main() {  }
