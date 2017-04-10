#![feature(plugin, custom_derive)]
#![plugin(rocket_codegen)]

extern crate crossbeam;
extern crate rocket;

use crossbeam::sync::MsQueue;
use rocket::State;

#[derive(FromForm, Debug)]
struct Event {
    description: String
}

struct LogChannel(MsQueue<Event>);

#[get("/push?<event>")]
fn push(event: Event, queue: State<LogChannel>) -> &'static str {
    queue.0.push(event);
    "got it"
}

#[get("/pop")]
fn pop(queue: State<LogChannel>) -> String {
    let e = queue.0.pop();
    e.description
}

// Use with: curl http://<rocket ip>:8000/test?foo=bar

fn main() {
    let q:MsQueue<Event> = MsQueue::new();

    rocket::ignite()
        .mount("/", routes![push,pop])
        .manage(LogChannel(q))
        .launch();

}

#[cfg(test)]
mod test {
    use super::rocket;
    use rocket::testing::MockRequest;
    use rocket::http::Status;
    use rocket::http::Method::*;
    use crossbeam::sync::MsQueue;
    use std::{thread, time};
    use super::LogChannel;
    use super::Event;

    #[test]
    fn test_get() {
        let q: MsQueue<Event> = MsQueue::new();
        let rocket = rocket::ignite().manage(LogChannel(q)).mount("/", routes![super::push, super::pop]);
        let mut req = MockRequest::new(Get, "/push?description=test1");
        let response = req.dispatch_with(&rocket);
        assert_eq!(response.status(), Status::Ok);

        let ten_millis = time::Duration::from_millis(10);
        thread::sleep(ten_millis);

        let mut req = MockRequest::new(Get, "/pop");
        let mut response = req.dispatch_with(&rocket);
        let body_str = response.body().and_then(|body| body.into_string());
        assert_eq!(body_str, Some("test1".to_string()));
    }
}