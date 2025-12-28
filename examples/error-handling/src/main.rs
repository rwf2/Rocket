#[macro_use] extern crate rocket;

#[cfg(test)] mod tests;

use std::num::ParseIntError;

use rocket::{Rocket, Build, Responder};
use rocket::response::{content, status};
use rocket::http::{Status, uri::Origin};

use rocket::serde::{Serialize, json::Json};
use rocket::request::FromParamError;

#[get("/hello/<name>/<age>")]
fn hello(name: &str, age: i8) -> String {
    format!("Hello, {} year old named {}!", age, name)
}

#[get("/<code>")]
fn forced_error(code: u16) -> Status {
    Status::new(code)
}

// TODO: Derive TypedError
#[derive(TypedError, Debug)]
struct CustomError;

#[get("/")]
fn forced_custom_error() -> Result<(), CustomError> {
    Err(CustomError)
}

#[catch(500, error = "<_e>")]
fn catch_custom(_e: &CustomError) -> &'static str {
    "You found the custom error!"
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

// Code to generate a Json response:
#[derive(Responder)]
#[response(status = 422)]
struct ParameterError<T>(T);

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct ErrorInfo<'a> {
    invalid_value: &'a str,
    description: String,
}

// Actual catcher:
#[catch(422, error = "<int_error>")]
fn param_error<'a>(
    // `&ParseIntError` would also work here, but `&FromParamError<ParseIntError>`
    // also gives us access to `raw`, the specific segment that failed to parse.
    int_error: &FromParamError<'a, ParseIntError>
) -> ParameterError<Json<ErrorInfo<'a>>> {
    ParameterError(Json(ErrorInfo {
        invalid_value: int_error.raw,
        description: format!("{}", int_error.error),
    }))
}

#[catch(default)]
fn sergio_error() -> &'static str {
    "I...don't know what to say."
}

#[catch(default)]
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
        .mount("/", routes![hello, forced_error, forced_custom_error])
        .register("/", catchers![general_not_found, default_catcher, catch_custom])
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
