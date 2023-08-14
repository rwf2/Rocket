use rocket_sync_db_pools::database;

struct Connection;
struct Manager;

use rocket::{Rocket, Build};
use rocket_sync_db_pools::{r2d2, Poolable, PoolResult};

impl r2d2::ManageConnection for Manager {
    type Connection = Connection;
    type Error = std::convert::Infallible;

    fn connect(&self) -> Result<Self::Connection, Self::Error> { Ok(Connection) }
    fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> { Ok(()) }
    fn has_broken(&self, conn: &mut Self::Connection) -> bool { true }
}

impl Poolable for Connection {
    type Manager = Manager;
    type Error = std::convert::Infallible;

    fn pool(db_name: &str, rocket: &Rocket<Build>) -> PoolResult<Self> {
        todo!()
    }
}

#[database]
struct A(Connection);

#[database(1)]
struct B(Connection);

#[database(123)]
struct C(Connection);

#[database("hello" "hi")]
struct D(Connection);

#[database("test")]
enum Foo {  }

#[database("test")]
struct Bar(Connection, Connection);

#[database("test")]
union Baz {  }

#[database("test")]
struct E<'r>(&'r str);

#[database("test")]
struct F<T>(T);

fn main() {  }
