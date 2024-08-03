use rocket::request::FromParam;

#[derive(FromParam)]
struct Foo1 {
    a: String
}

#[derive(FromParam)]
struct Foo2 {}

#[derive(FromParam)]
enum Foo3 {
    A(String),
    B(String)
}

#[derive(FromParam)]
struct Foo4(usize);

fn main() {}
