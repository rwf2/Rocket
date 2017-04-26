#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

use std::io::Cursor;

use rocket::Fairing;
use rocket::http::Method;

#[cfg(test)] mod tests;

#[put("/")]
fn hello() -> &'static str {
    "Hello, world!"
}

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/", routes![hello])
        .attach(Fairing::Launch(Box::new(|rocket| {
            println!("Rocket is about to launch! Exciting! Here we go...");
            Ok(rocket)
        })))
        .attach(Fairing::Request(Box::new(|req, _| {
            println!("    => Incoming request: {}", req);
            println!("    => Changing method to `PUT`.");
            req.set_method(Method::Put);
        })))
        .attach(Fairing::Response(Box::new(|_, res| {
            println!("    => Rewriting response body.");
            res.set_sized_body(Cursor::new("Hello, fairings!"));
        })))
}

fn main() {
    rocket().launch();
}
