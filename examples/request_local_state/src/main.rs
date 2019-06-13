#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

use std::sync::atomic::{AtomicUsize, Ordering};

use rocket::request::{self, Request, FromRequest, State};
use rocket::outcome::Outcome::*;

#[cfg(test)] mod tests;

#[derive(Default)]
struct Atomics {
    uncached: AtomicUsize,
    cached: AtomicUsize,
}

struct Guard1;
struct Guard2;

impl<'a, 'r> FromRequest<'a, 'r> for Guard1 {
    type Error = ();

    fn from_request(req: &'a Request<'r>) -> request::Outcome<Self, ()> {
        let atomics = req.guard::<State<'_, Atomics>>()?;
        atomics.uncached.fetch_add(1, Ordering::Relaxed);
        req.local_cache(|| atomics.cached.fetch_add(1, Ordering::Relaxed));

        Success(Guard1)
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for Guard2 {
    type Error = ();

    fn from_request(req: &'a Request<'r>) -> request::Outcome<Self, ()> {
        req.guard::<Guard1>()?;
        Success(Guard2)
    }
}

#[get("/")]
fn index(_g1: Guard1, _g2: Guard2) {
    // This exists only to run the request guards.
}

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .manage(Atomics::default())
        .mount("/", routes!(index))
}

fn main() {
    rocket().launch();
}
