extern crate rocket;

#[cfg(test)]
mod tests;

use std::io;
use std::fs::File;

use rocket::{Request, Route, Data, Catcher, Error};
use rocket::http::Status;
use rocket::request::FromParam;
use rocket::response::{self, Responder};
use rocket::response::status::Custom;
use rocket::handler::Outcome;
use rocket::http::Method::*;

fn forward(_req: &Request, data: Data) -> Outcome<'static> {
    Outcome::forward(data)
}

fn hi(_req: &Request, _: Data) -> Outcome<'static> {
    Outcome::of("Hello!")
}

fn name<'a>(req: &'a Request, _: Data) -> Outcome<'a> {
    Outcome::of(req.get_param(0).unwrap_or("unnamed"))
}

fn echo_url(req: &Request, _: Data) -> Outcome<'static> {
    let param = req.uri()
        .as_str()
        .split_at(6)
        .1;
    Outcome::of(String::from_param(param).unwrap())
}

fn upload<'r>(req: &'r Request, data: Data) -> Outcome<'r> {
    if !req.content_type().map_or(false, |ct| ct.is_plain()) {
        println!("    => Content-Type of upload must be text/plain. Ignoring.");
        return Outcome::failure(Status::BadRequest);
    }

    let file = File::create("/tmp/upload.txt");
    if let Ok(mut file) = file {
        if let Ok(n) = io::copy(&mut data.open(), &mut file) {
            return Outcome::of(format!("OK: {} bytes uploaded.", n));
        }

        println!("    => Failed copying.");
        Outcome::failure(Status::InternalServerError)
    } else {
        println!("    => Couldn't open file: {:?}", file.unwrap_err());
        Outcome::failure(Status::InternalServerError)
    }
}

fn get_upload(_: &Request, _: Data) -> Outcome<'static> {
    Outcome::of(File::open("/tmp/upload.txt").ok())
}

fn not_found_handler<'r>(_: Error, req: &'r Request) -> response::Result<'r> {
    Custom(Status::NotFound, format!("Couldn't find: {}", req.uri())).respond()
}

fn rocket() -> rocket::Rocket {
    let always_forward = Route::ranked(1, Get, "/", forward);
    let hello = Route::ranked(2, Get, "/", hi);

    let echo = Route::new(Get, "/echo:<str>", echo_url);
    let name = Route::new(Get, "/<name>", name);
    let post_upload = Route::new(Post, "/", upload);
    let get_upload = Route::new(Get, "/", get_upload);

    let not_found_catcher = Catcher::new(404, not_found_handler);

    rocket::ignite()
        .mount("/", vec![always_forward, hello, echo])
        .mount("/upload", vec![get_upload, post_upload])
        .mount("/hello", vec![name.clone()])
        .mount("/hi", vec![name])
        .catch(vec![not_found_catcher])
}

fn main() {
    rocket().launch();
}
