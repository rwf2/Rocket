#![feature(proc_macro_hygiene)]

#[macro_use] extern crate rocket;

use std::sync::atomic::{AtomicUsize, Ordering};

use rocket::outcome::Outcome::*;
use rocket::request::{self, FromRequest, FromRequestAsync, FromRequestFuture, Request, State};

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

impl<'a, 'r> FromRequest<'a, 'r> for Guard1 {
    type Error = ();

    fn from_request(req: &'a Request<'r>) -> request::Outcome<Self, ()> {
        let atomics = try_outcome!(req.guard::<State<'_, Atomics>>());
        atomics.uncached.fetch_add(1, Ordering::Relaxed);
        req.local_cache(|| atomics.cached.fetch_add(1, Ordering::Relaxed));

        Success(Guard1)
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for Guard2 {
    type Error = ();

    fn from_request(req: &'a Request<'r>) -> request::Outcome<Self, ()> {
        try_outcome!(req.guard::<Guard1>());
        Success(Guard2)
    }
}

impl<'a, 'r> FromRequestAsync<'a, 'r> for Guard3 {
    type Error = ();

    fn from_request<'fut>(req: &'a Request<'r>) -> FromRequestFuture<'fut, Self, ()>
        where 'a: 'fut
    {
        Box::pin(async move {
            let atomics = try_outcome!(req.guard::<State<'_, Atomics>>());
            atomics.uncached.fetch_add(1, Ordering::Relaxed);
            req.local_cache_async(async {
                atomics.cached.fetch_add(1, Ordering::Relaxed)
            }).await;

            Success(Guard3)
        })
    }
}

impl<'a, 'r> FromRequestAsync<'a, 'r> for Guard4 {
    type Error = ();

    fn from_request<'fut>(req: &'a Request<'r>) -> FromRequestFuture<'fut, Self, ()>
        where 'a: 'fut
    {
        Box::pin(async move {
            try_outcome!(Guard3::from_request(req).await);
            Success(Guard4)
        })
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

fn main() {
    let _ = rocket().launch();
}
