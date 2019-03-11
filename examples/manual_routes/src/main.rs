extern crate rocket;

#[cfg(test)]
mod tests;

use std::{io, env};
use std::fs::File;

use rocket::{Request, Handler, Route, Data, Catcher};
use rocket::http::{Status, RawStr};
use rocket::response::{self, Responder, status::Custom};
use rocket::handler::Outcome;
use rocket::outcome::IntoOutcome;
use rocket::http::Method::*;

fn forward<'r>(_req: &'r Request, data: Data) -> Outcome<'r> {
    Outcome::forward(data)
}

fn hi<'r>(req: &'r Request, _: Data) -> Outcome<'r> {
    Outcome::from(req, "Hello!")
}

fn name<'a>(req: &'a Request, _: Data) -> Outcome<'a> {
    let param = req.get_param::<&'a RawStr>(0)
        .and_then(|res| res.ok())
        .unwrap_or("unnamed".into());

    Outcome::from(req, param.as_str())
}

fn echo_url<'r>(req: &'r Request, _: Data) -> Outcome<'r> {
    let param = req.get_param::<&RawStr>(1)
        .and_then(|res| res.ok())
        .into_outcome(Status::BadRequest)?;

    Outcome::from(req, RawStr::from_str(param).url_decode())
}

fn upload<'r>(req: &'r Request, data: Data) -> Outcome<'r> {
    if !req.content_type().map_or(false, |ct| ct.is_plain()) {
        println!("    => Content-Type of upload must be text/plain. Ignoring.");
        return Outcome::failure(Status::BadRequest);
    }

    let file = File::create(env::temp_dir().join("upload.txt"));
    if let Ok(mut file) = file {
        if let Ok(n) = io::copy(&mut data.open(), &mut file) {
            return Outcome::from(req, format!("OK: {} bytes uploaded.", n));
        }

        println!("    => Failed copying.");
        Outcome::failure(Status::InternalServerError)
    } else {
        println!("    => Couldn't open file: {:?}", file.unwrap_err());
        Outcome::failure(Status::InternalServerError)
    }
}

fn get_upload<'r>(req: &'r Request, _: Data) -> Outcome<'r> {
    Outcome::from(req, File::open(env::temp_dir().join("upload.txt")).ok())
}

fn not_found_handler<'r>(req: &'r Request) -> response::Result<'r> {
    let res = Custom(Status::NotFound, format!("Couldn't find: {}", req.uri()));
    res.respond_to(req)
}

#[derive(Clone)]
struct CustomHandler {
    data: &'static str
}

impl CustomHandler {
    fn new(data: &'static str) -> Vec<Route> {
        vec![Route::new(Get, "/<id>", Self { data })]
    }
}

impl Handler for CustomHandler {
    fn handle<'r>(&self, req: &'r Request, data: Data) -> Outcome<'r> {
        let id = req.get_param::<&RawStr>(0)
            .and_then(|res| res.ok())
            .or_forward(data)?;

        Outcome::from(req, format!("{} - {}", self.data, id))
    }
}

fn rocket() -> rocket::Rocket {
    let always_forward = Route::ranked(1, Get, "/", forward);
    let hello = Route::ranked(2, Get, "/", hi);

    let echo = Route::new(Get, "/echo/<str>", echo_url);
    let name = Route::new(Get, "/<name>", name);
    let post_upload = Route::new(Post, "/", upload);
    let get_upload = Route::new(Get, "/", get_upload);

    let not_found_catcher = Catcher::new(404, not_found_handler);

    rocket::ignite()
        .mount("/", vec![always_forward, hello, echo])
        .mount("/upload", vec![get_upload, post_upload])
        .mount("/hello", vec![name.clone()])
        .mount("/hi", vec![name])
        .mount("/custom", CustomHandler::new("some data here"))
        .register(vec![not_found_catcher])
}

fn main() {
    rocket().launch();
}
