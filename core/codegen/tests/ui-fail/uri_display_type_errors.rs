#[macro_use] extern crate rocket;

struct BadType;

#[derive(UriDisplayQuery)]
struct Bar1(BadType);

#[derive(UriDisplayQuery)]
struct Bar2 {
    field: BadType,
}

#[derive(UriDisplayQuery)]
struct Bar3 {
    field: String,
    bad: BadType,
}

#[derive(UriDisplayQuery)]
enum Bar4 {
    Inner(BadType),
}

#[derive(UriDisplayQuery)]
enum Bar5 {
    Inner {
        field: BadType,
    },
}

#[derive(UriDisplayQuery)]
enum Bar6 {
    Inner {
        field: String,
        other: BadType,
    },
}

#[derive(UriDisplayPath)]
struct Baz(BadType);

fn main() {  }
