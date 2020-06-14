#![feature(proc_macro_hygiene)]

#[macro_use] extern crate rocket;

use std::sync::atomic::{AtomicUsize, Ordering};

use rocket::outcome::Outcome::*;
use rocket::request::{self, FromRequest, Request, State};

#[cfg(test)] mod tests;

#[derive(Default)]
struct Atomics {
    uncached: AtomicUsize,
    cached: AtomicUsize,
}

struct Guard1;
struct Guard2;
struct Guard3;
struct Guard4;

#[rocket::async_trait]
impl<'a, 'r> FromRequest<'a, 'r> for Guard1 {
    type Error = ();

    async fn from_request(req: &'a Request<'r>) -> request::Outcome<Self, ()> {
        let atomics = try_outcome!(req.guard::<State<'_, Atomics>>().await);
        atomics.uncached.fetch_add(1, Ordering::Relaxed);
        req.local_cache(|| atomics.cached.fetch_add(1, Ordering::Relaxed));

        Success(Guard1)
    }
}

#[rocket::async_trait]
impl<'a, 'r> FromRequest<'a, 'r> for Guard2 {
    type Error = ();

    async fn from_request(req: &'a Request<'r>) -> request::Outcome<Self, ()> {
        try_outcome!(req.guard::<Guard1>().await);
        Success(Guard2)
    }
}

#[rocket::async_trait]
impl<'a, 'r> FromRequest<'a, 'r> for Guard3 {
    type Error = ();

    async fn from_request(req: &'a Request<'r>) -> request::Outcome<Self, ()> {
        let atomics = try_outcome!(req.guard::<State<'_, Atomics>>().await);
        atomics.uncached.fetch_add(1, Ordering::Relaxed);
        req.local_cache_async(async {
            atomics.cached.fetch_add(1, Ordering::Relaxed)
        }).await;

        Success(Guard3)
    }
}

#[rocket::async_trait]
impl<'a, 'r> FromRequest<'a, 'r> for Guard4 {
    type Error = ();

    async fn from_request(req: &'a Request<'r>) -> request::Outcome<Self, ()> {
        try_outcome!(Guard3::from_request(req).await);
        Success(Guard4)
    }
}

#[get("/sync")]
fn r_sync(_g1: Guard1, _g2: Guard2) {
    // This exists only to run the request guards.
}

#[get("/async")]
async fn r_async(_g1: Guard3, _g2: Guard4) {
    // This exists only to run the request guards.
}

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .manage(Atomics::default())
        .mount("/", routes![r_sync, r_async])
}

#[rocket::main]
async fn main() {
    let _ = rocket().launch().await;
}
