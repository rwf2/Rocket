#[macro_use] extern crate rocket;

use std::sync::atomic::{AtomicUsize, Ordering};

use rocket::{Rocket, State, Build};
use rocket::fairing::AdHoc;
use rocket::http::Method;

#[derive(Default)]
struct Counter {
    attach: AtomicUsize,
    get: AtomicUsize,
}

#[get("/")]
fn index(counter: &State<Counter>) -> String {
    let attaches = counter.attach.load(Ordering::Relaxed);
    let gets = counter.get.load(Ordering::Acquire);
    format!("{}, {}", attaches, gets)
}

fn rocket() -> Rocket<Build> {
    rocket::build()
        .mount("/", routes![index])
        .attach(AdHoc::on_ignite("Outer", |rocket| async {
            let counter = Counter::default();
            counter.attach.fetch_add(1, Ordering::Relaxed);
            let rocket = rocket.manage(counter)
                .attach(AdHoc::on_request("Inner", |req, _| {
                    Box::pin(async move {
                        if req.method() == Method::Get {
                            let counter = req.rocket().state::<Counter>().unwrap();
                            counter.get.fetch_add(1, Ordering::Release);
                        }
                    })
                }));

            rocket
        }))
}

mod nested_fairing_attaches_tests {
    use super::*;
    use rocket::local::blocking::Client;

    #[test]
    fn test_counts() {
        let client = Client::debug(rocket()).unwrap();
        let response = client.get("/").dispatch();
        assert_eq!(response.into_string(), Some("1, 1".into()));

        let response = client.get("/").dispatch();
        assert_eq!(response.into_string(), Some("1, 2".into()));

        client.get("/").dispatch();
        client.get("/").dispatch();
        let response = client.get("/").dispatch();
        assert_eq!(response.into_string(), Some("1, 5".into()));
    }
}
