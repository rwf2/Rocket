extern crate rocket;
#[macro_use] extern crate rocket_contrib;

struct Unknown;

#[database("foo")]
struct A(Unknown);
//~^ ERROR Unknown: rocket_contrib::databases::Poolable

#[database("foo")]
struct B(Vec<i32>);
//~^ ERROR Vec<i32>: rocket_contrib::databases::Poolable

fn main() {  }
