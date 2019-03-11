#[macro_use] extern crate rocket_contrib;

use rocket_contrib::databases::diesel;

#[database]
//~^ ERROR expected string literal
struct A(diesel::SqliteConnection);

#[database(1)]
//~^ ERROR expected string literal
struct B(diesel::SqliteConnection);

#[database(123)]
//~^ ERROR expected string literal
struct C(diesel::SqliteConnection);

#[database("hello" "hi")]
//~^ ERROR expected string literal
struct D(diesel::SqliteConnection);

#[database("test")]
enum Foo {  }
//~^ ERROR on structs

#[database("test")]
struct Bar(diesel::SqliteConnection, diesel::SqliteConnection);
//~^ ERROR one unnamed field

#[database("test")]
union Baz {  }
//~^ ERROR on structs

#[database("test")]
struct E<'r>(&'r str);
//~^ ERROR generics

#[database("test")]
struct F<T>(T);
//~^ ERROR generics

fn main() {  }
