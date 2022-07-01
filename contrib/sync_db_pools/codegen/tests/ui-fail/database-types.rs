#[macro_use] extern crate rocket_sync_db_pools;

struct Unknown;

#[database("foo")]
struct A(Unknown);

#[database("foo")]
struct B(Vec<i32>);

fn main() {  }
