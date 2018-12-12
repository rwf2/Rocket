extern crate rocket;
#[macro_use] extern crate rocket_contrib;

struct Unknown;

#[database("foo")]
struct A(Unknown);
//~^ ERROR Unknown: rocket_contrib::databases::Poolable

fn main() {  }
