#[macro_use] extern crate rocket;

#[cfg(test)] mod tests;

use rocket::{Rocket, Build};
use rocket::response::{content, status};
use rocket::http::{Status, uri::Origin};

// Custom impl so I can implement Static (or Transient) ---
// We should upstream implementations for most common error types
// in transient itself
use rocket::catcher::{Static};
use std::num::ParseIntError;

#[derive(Debug)]
#[allow(unused)]
struct IntErr(ParseIntError);
impl Static for IntErr {}

struct I8(i8);
use rocket::request::FromParam;
impl FromParam<'_> for I8 {
    type Error = IntErr;
    fn from_param(param: &str) -> Result<Self, Self::Error> {
        param.parse::<i8>().map(Self).map_err(IntErr)
    }
}
// ------------------------------

#[get("/hello/<name>/<age>")]
fn hello(name: &str, age: I8) -> String {
    format!("Hello, {} year old named {}!", age.0, name)
}

#[get("/<code>")]
fn forced_error(code: u16) -> Status {
    Status::new(code)
}

#[catch(404)]
fn general_not_found() -> content::RawHtml<&'static str> {
    content::RawHtml(r#"
        <p>Hmm... What are you looking for?</p>
        Say <a href="/hello/Sergio/100">hello!</a>
    "#)
}

#[catch(404)]
fn hello_not_found(uri: &Origin<'_>) -> content::RawHtml<String> {
    content::RawHtml(format!("\
        <p>Sorry, but '{}' is not a valid path!</p>\
        <p>Try visiting /hello/&lt;name&gt;/&lt;age&gt; instead.</p>",
        uri))
}

// Demonstrates a downcast error from `hello`
// NOTE: right now, the error must be the first parameter, and all three params must
// be present. I'm thinking about adding a param to the macro to indicate which (and whether)
// param is a downcast error.

// `error` and `status` type. All other params must be `FromOrigin`?
#[catch(422, error = "<e>" /*, status = "<_s>"*/)]
fn param_error(e: &IntErr, uri: &Origin<'_>) -> content::RawHtml<String> {
    content::RawHtml(format!("\
        <p>Sorry, but '{}' is not a valid path!</p>\
        <p>Try visiting /hello/&lt;name&gt;/&lt;age&gt; instead.</p>\
        <p>Error: {e:?}</p>",
        uri))
}

#[catch(default)]
fn sergio_error() -> &'static str {
    "I...don't know what to say."
}

#[catch(default, status = "<status>")]
fn default_catcher(status: Status, uri: &Origin<'_>) -> status::Custom<String> {
    let msg = format!("{} ({})", status, uri);
    status::Custom(status, msg)
}

#[allow(dead_code)]
#[get("/unmanaged")]
fn unmanaged(_u8: &rocket::State<u8>, _string: &rocket::State<String>) { }

fn rocket() -> Rocket<Build> {
    rocket::build()
        // .mount("/", routes![hello, hello]) // uncomment this to get an error
        // .mount("/", routes![unmanaged]) // uncomment this to get a sentinel error
        .mount("/", routes![hello, forced_error])
        .register("/", catchers![general_not_found, default_catcher])
        .register("/hello", catchers![hello_not_found, param_error])
        .register("/hello/Sergio", catchers![sergio_error])
}

#[rocket::main]
async fn main() {
    if let Err(e) = rocket().launch().await {
        println!("Whoops! Rocket didn't launch!");
        // We drop the error to get a Rocket-formatted panic.
        drop(e);
    };
}
