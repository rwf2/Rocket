#[macro_use] extern crate rocket;

struct BadType;

#[derive(UriDisplayQuery)]
struct Bar1(BadType);
//~^ ERROR UriDisplay<rocket::http::uri::Query>

#[derive(UriDisplayQuery)]
struct Bar2 {
    field: BadType,
    //~^ ERROR UriDisplay<rocket::http::uri::Query>
}

#[derive(UriDisplayQuery)]
struct Bar3 {
    field: String,
    bad: BadType,
    //~^ ERROR UriDisplay<rocket::http::uri::Query>
}

#[derive(UriDisplayQuery)]
enum Bar4 {
    Inner(BadType),
    //~^ ERROR UriDisplay<rocket::http::uri::Query>
}

#[derive(UriDisplayQuery)]
enum Bar5 {
    Inner {
        field: BadType,
        //~^ ERROR UriDisplay<rocket::http::uri::Query>
    },
}

#[derive(UriDisplayQuery)]
enum Bar6 {
    Inner {
        field: String,
        other: BadType,
        //~^ ERROR UriDisplay<rocket::http::uri::Query>
    },
}

#[derive(UriDisplayPath)]
struct Baz(BadType);
//~^ ERROR UriDisplay<rocket::http::uri::Path>

fn main() {  }
