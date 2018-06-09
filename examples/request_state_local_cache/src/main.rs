#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]
extern crate rocket;

use std::thread;
use std::time::Duration;

use rocket::request::{self, Request, FromRequest};
use rocket::outcome::Outcome::*;

#[cfg(test)] mod tests;

#[derive(Clone, Copy)]
struct Sleep();
struct SleepCached(Sleep);
struct ForwardGuard();

impl<'a, 'r> FromRequest<'a, 'r> for ForwardGuard {
    type Error = ();

    fn from_request(_: &'a Request<'r>) -> request::Outcome<Self, ()> {
        Forward(())
    }
}

impl Sleep {
    fn get(_: &Request) -> Sleep {
        thread::sleep(Duration::from_secs(3));
        Sleep()
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for Sleep {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, ()> {
        Success(Sleep::get(request))
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for SleepCached {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, ()> {
        let s = request.local_cache(Sleep::get);
        Success(SleepCached(*s))
    }
}

#[get("/")]
fn index(_s: Sleep, _f: ForwardGuard) {
}

#[get("/", rank = 2)]
fn index2(_s: Sleep) {
}

#[get("/cached")]
fn index_cached(_s: SleepCached, _f: ForwardGuard) {
}

#[get("/cached", rank = 2)]
fn index_cached2(_s: SleepCached) {
}

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/", routes!(index, index2, index_cached, index_cached2))
}

fn main() {
    rocket().launch();
}
