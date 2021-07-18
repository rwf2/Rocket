#[macro_use] extern crate rocket_db_pools;

struct Unknown;

#[derive(Database)]
#[database("foo")]
struct A(Unknown);

#[derive(Database)]
#[database("bar")]
struct B(Vec<i32>);

fn main() {  }
