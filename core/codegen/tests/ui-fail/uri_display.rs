// normalize-stderr-test: "::: (.*)/core/http" -> "::: $$ROCKET/core/http"

#[macro_use] extern crate rocket;

#[derive(UriDisplayQuery)]
//~^ ERROR Foo1: rocket::http::uri::UriDisplay<rocket::http::uri::Query>
//~| ERROR Foo1: rocket::http::uri::UriDisplay<rocket::http::uri::Query>
//~| ERROR Foo1: rocket::http::uri::UriDisplay<rocket::http::uri::Query>
struct Foo1;
//~^ ERROR not supported

#[derive(UriDisplayQuery)]
//~^ ERROR Foo2: rocket::http::uri::UriDisplay<rocket::http::uri::Query>
//~| ERROR Foo2: rocket::http::uri::UriDisplay<rocket::http::uri::Query>
//~| ERROR Foo2: rocket::http::uri::UriDisplay<rocket::http::uri::Query>
struct Foo2();
//~^ ERROR not supported

#[derive(UriDisplayQuery)]
//~^ ERROR Foo3: rocket::http::uri::UriDisplay<rocket::http::uri::Query>
//~| ERROR Foo3: rocket::http::uri::UriDisplay<rocket::http::uri::Query>
//~| ERROR Foo3: rocket::http::uri::UriDisplay<rocket::http::uri::Query>
enum Foo3 { }
//~^ ERROR not supported

#[derive(UriDisplayQuery)]
//~^ ERROR Foo4: rocket::http::uri::UriDisplay<rocket::http::uri::Query>
//~| ERROR Foo4: rocket::http::uri::UriDisplay<rocket::http::uri::Query>
//~| ERROR Foo4: rocket::http::uri::UriDisplay<rocket::http::uri::Query>
enum Foo4 {
    Variant,
    //~^ ERROR not supported
}

#[derive(UriDisplayQuery)]
//~^ ERROR Foo5: rocket::http::uri::UriDisplay<rocket::http::uri::Query>
//~| ERROR Foo5: rocket::http::uri::UriDisplay<rocket::http::uri::Query>
//~| ERROR Foo5: rocket::http::uri::UriDisplay<rocket::http::uri::Query>
struct Foo5(String, String);
//~^ ERROR exactly one

#[derive(UriDisplayQuery)]
//~^ ERROR Foo6: rocket::http::uri::UriDisplay<rocket::http::uri::Query>
//~| ERROR Foo6: rocket::http::uri::UriDisplay<rocket::http::uri::Query>
//~| ERROR Foo6: rocket::http::uri::UriDisplay<rocket::http::uri::Query>
struct Foo6 {
    #[form(field = 123)]
    //~^ ERROR invalid value: expected string
    field: String,
}

#[derive(UriDisplayPath)]
//~^ ERROR Foo7: rocket::http::uri::UriDisplay<rocket::http::uri::Path>
//~| ERROR Foo7: rocket::http::uri::UriDisplay<rocket::http::uri::Path>
struct Foo7(String, usize);
//~^ ERROR exactly one

#[derive(UriDisplayPath)]
//~^ ERROR Foo8: rocket::http::uri::UriDisplay<rocket::http::uri::Path>
//~| ERROR Foo8: rocket::http::uri::UriDisplay<rocket::http::uri::Path>
struct Foo8;
//~^ ERROR exactly one

#[derive(UriDisplayPath)]
//~^ ERROR Foo9: rocket::http::uri::UriDisplay<rocket::http::uri::Path>
//~| ERROR Foo9: rocket::http::uri::UriDisplay<rocket::http::uri::Path>
enum Foo9 {  }
//~^ ERROR not supported

#[derive(UriDisplayPath)]
//~^ ERROR Foo10: rocket::http::uri::UriDisplay<rocket::http::uri::Path>
//~| ERROR Foo10: rocket::http::uri::UriDisplay<rocket::http::uri::Path>
struct Foo10 {
//~^ ERROR not supported
    named: usize
}

fn main() { }
