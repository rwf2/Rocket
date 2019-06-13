#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

#[cfg(test)] mod tests;

use rocket::State;
use crossbeam::queue::SegQueue;

struct LogChannel(SegQueue<String>);

#[put("/push?<event>")]
fn push(event: String, queue: State<'_, LogChannel>) {
    queue.0.push(event);
}

#[get("/pop")]
fn pop(queue: State<'_, LogChannel>) -> Option<String> {
    queue.0.pop().ok()
}

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/", routes![push, pop])
        .manage(LogChannel(SegQueue::new()))
}

fn main() {
    rocket().launch();
}
