#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

#[cfg(test)] mod tests;

use std::sync::atomic::{AtomicUsize, Ordering};

use rocket::State;
use rocket::response::content;

struct HitCount(AtomicUsize);

#[get("/")]
fn index(hit_count: State<'_, HitCount>) -> content::Html<String> {
    hit_count.0.fetch_add(1, Ordering::Relaxed);
    let msg = "Your visit has been recorded!";
    let count = format!("Visits: {}", count(hit_count));
    content::Html(format!("{}<br /><br />{}", msg, count))
}

#[get("/count")]
fn count(hit_count: State<'_, HitCount>) -> String {
    hit_count.0.load(Ordering::Relaxed).to_string()
}

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/", routes![index, count])
        .manage(HitCount(AtomicUsize::new(0)))
}

fn main() {
    rocket().launch();
}
