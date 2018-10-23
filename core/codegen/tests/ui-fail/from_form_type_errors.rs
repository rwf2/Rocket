#[macro_use] extern crate rocket;

struct Unknown;

#[derive(FromForm)]
struct BadType3 {
    field: Unknown,
    //~^ rocket::request::FromFormValue
}

struct Foo<T>(T);

#[derive(FromForm)]
struct Other {
    field: Foo<usize>,
    //~^ rocket::request::FromFormValue
}

fn main() {  }
