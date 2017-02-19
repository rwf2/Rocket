#![feature(plugin, custom_derive)]
#![plugin(rocket_codegen)]

extern crate crossbeam;
extern crate rocket;

use crossbeam::scope;
use crossbeam::sync::MsQueue;
use rocket::State;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const THREAD_SLEEP: u64 = 500;

#[derive(FromForm, Debug)]
struct Event {
    foo: String
}

struct LogChannel(Arc<MsQueue<Event>>);

#[get("/test?<event>")]
fn test(event: Event, queue: State<LogChannel>) -> &'static str {
    queue.0.push(event);
    "got it"
}

// Use with: curl http://<rocket ip>:8000/test?foo=bar

fn main() {
    let q: Arc<MsQueue<Event>> = Arc::new(MsQueue::new());

    scope(|scope| {
        scope.spawn(|| {
            loop {
                match q.try_pop() {
                    //This is a synchronous stream of received events
                    Some(e) => println!("Do something useful: {:?}", e),
                    _ => {
                        thread::sleep(Duration::from_millis(THREAD_SLEEP))
                    }
                }
            }
        });

        scope.spawn(|| {
            rocket::ignite()
                .mount("/", routes![test])
                .manage(LogChannel(q.clone()))
                .launch();
        });
    });
}

#[cfg(test)]
mod test {
    use super::rocket;
    use rocket::testing::MockRequest;
    use rocket::http::Status;
    use rocket::http::Method::*;
    use crossbeam::sync::MsQueue;
    use std::sync::Arc;
    use super::LogChannel;
    use super::Event;

    #[test]
    fn test_get() {
        let q: Arc<MsQueue<Event>> = Arc::new(MsQueue::new());
        let rocket = rocket::ignite().manage(LogChannel(q.clone())).mount("/", routes![super::test]);
        let mut req = MockRequest::new(Get, "/test?foo=bar");
        let response = req.dispatch_with(&rocket);
        assert_eq!(response.status(), Status::Ok);
    }
}
