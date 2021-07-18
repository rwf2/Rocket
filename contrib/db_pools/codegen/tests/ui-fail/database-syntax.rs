use rocket_db_pools::{deadpool_postgres, Database};

#[derive(Database)]
#[database(123)]
struct A(deadpool_postgres::Pool);

#[derive(Database)]
#[database("some-name", "another")]
struct B(deadpool_postgres::Pool);

#[derive(Database)]
#[database("some-name", name = "another")]
struct C(deadpool_postgres::Pool);

#[derive(Database)]
#[database("foo")]
enum D {  }

#[derive(Database)]
struct E(deadpool_postgres::Pool);

#[derive(Database)]
#[database("foo")]
struct F;

#[derive(Database)]
#[database("foo")]
struct G(deadpool_postgres::Pool, deadpool_postgres::Pool);

#[derive(Database)]
#[database("foo")]
struct H {
    foo: deadpool_postgres::Pool,
}

fn main() {  }
