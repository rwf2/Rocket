#[macro_use] extern crate rocket;

#[derive(Responder)]
struct Thing1 {
    thing: u8,
}

#[derive(Responder)]
struct Thing2 {
    thing: String,
    other: u8,
}

#[derive(Responder)]
struct Thing3 {
    thing: u8,
    other: u8,
}

#[derive(Responder)]
struct Thing4 {
    thing: String,
    other: rocket::http::ContentType,
    then: String,
}

#[get("/")]
fn foo() -> usize { 0 }

fn main() {  }
