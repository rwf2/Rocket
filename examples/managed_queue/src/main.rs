#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
extern crate crossbeam;

#[cfg(test)] mod tests;

use rocket::State;
use crossbeam::queue::MsQueue;

struct LogChannel(MsQueue<String>);

#[put("/push?<event>")]
fn push(event: String, queue: State<LogChannel>) {
    queue.0.push(event);
}

#[get("/pop")]
fn pop(queue: State<LogChannel>) -> String {
    queue.0.pop()
}

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/", routes![push, pop])
        .manage(LogChannel(MsQueue::new()))
}

fn main() {
    rocket().launch();
}
