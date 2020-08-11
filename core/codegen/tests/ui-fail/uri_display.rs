#[macro_use] extern crate rocket;

#[derive(UriDisplayQuery)]
struct Foo1;

#[derive(UriDisplayQuery)]
struct Foo2();

#[derive(UriDisplayQuery)]
enum Foo3 { }

#[derive(UriDisplayQuery)]
enum Foo4 {
    Variant,
}

#[derive(UriDisplayQuery)]
struct Foo5(String, String);

#[derive(UriDisplayQuery)]
struct Foo6 {
    #[form(field = 123)]
    field: String,
}

#[derive(UriDisplayPath)]
struct Foo7(String, usize);

#[derive(UriDisplayPath)]
struct Foo8;

#[derive(UriDisplayPath)]
enum Foo9 {  }

#[derive(UriDisplayPath)]
struct Foo10 {
    named: usize
}

fn main() { }
