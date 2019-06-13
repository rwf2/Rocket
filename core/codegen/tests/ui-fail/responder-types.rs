// normalize-stderr-test: "<(.*) as (.*)>" -> "$1 as $$TRAIT"
// normalize-stderr-test: "and \d+ others" -> "and $$N others"

#[macro_use] extern crate rocket;

#[derive(Responder)]
struct Thing1 {
    thing: u8,
    //~^ ERROR Responder
}

#[derive(Responder)]
struct Thing2 {
    thing: String,
    other: u8,
    //~^ ERROR Header
}

#[derive(Responder)]
struct Thing3 {
    thing: u8,
    //~^ ERROR Responder
    other: u8,
    //~^ ERROR Header
}

#[derive(Responder)]
struct Thing4 {
    thing: String,
    other: rocket::http::ContentType,
    then: String,
    //~^ ERROR Header
}

fn main() {  }
