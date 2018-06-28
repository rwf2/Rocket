#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]
extern crate rocket;

use std::sync::atomic::{AtomicUsize, Ordering};

use rocket::request::{self, Request, FromRequest, State};
use rocket::outcome::Outcome::*;

#[cfg(test)] mod tests;

struct Atomics {
    first: AtomicUsize,
    second: AtomicUsize,
}

struct Guard1();
struct Guard2();

impl<'a, 'r> FromRequest<'a, 'r> for Guard1 {
    type Error = ();

    fn from_request(req: &'a Request<'r>) -> request::Outcome<Self, ()> {
        req.guard::<State<Atomics>>()?.first.fetch_add(1, Ordering::Relaxed);
        req.local_cache(|req| {
            req.guard::<State<Atomics>>().unwrap().second.fetch_add(1, Ordering::Relaxed);
        });

        Success(Guard1())
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for Guard2 {
    type Error = ();

    fn from_request(req: &'a Request<'r>) -> request::Outcome<Self, ()> {
        req.guard::<State<Atomics>>()?.first.fetch_add(1, Ordering::Relaxed);
        req.local_cache(|req| {
            req.guard::<State<Atomics>>().unwrap().second.fetch_add(1, Ordering::Relaxed);
        });

        Success(Guard2())
    }
}


#[get("/")]
fn index(_g1: Guard1, _g2: Guard2) {
}

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .manage(Atomics{
            first: AtomicUsize::new(0),
            second: AtomicUsize::new(0),
        })
        .mount("/", routes!(index))
}

fn main() {
    rocket().launch();
}
