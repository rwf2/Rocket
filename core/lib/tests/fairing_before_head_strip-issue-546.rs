#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

const RESPONSE_STRING: &'static str = "This is the body. Hello, world!";

#[head("/")]
fn head() -> &'static str {
    RESPONSE_STRING
}

#[get("/")]
fn auto() -> &'static str {
    RESPONSE_STRING
}

// Test that response fairings see the response body for all `HEAD` requests,
// whether they are auto-handled or not.
mod fairing_before_head_strip {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use rocket::fairing::AdHoc;
    use rocket::http::Method;
    use rocket::local::Client;
    use rocket::http::Status;
    use rocket::State;

    #[test]
    fn not_auto_handled() {
        let rocket = rocket::ignite()
            .mount("/", routes![head])
            .attach(AdHoc::on_request("Check HEAD", |req, _| {
                assert_eq!(req.method(), Method::Head);
            }))
            .attach(AdHoc::on_response("Check HEAD 2", |req, res| {
                assert_eq!(req.method(), Method::Head);
                assert_eq!(res.body_string(), Some(RESPONSE_STRING.into()));
            }));

        let client = Client::new(rocket).unwrap();
        let mut response = client.head("/").dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert!(response.body().is_none());
    }

    #[test]
    fn auto_handled() {
        #[derive(Default)]
        struct Counter(AtomicUsize);

        let counter = Counter::default();
        let rocket = rocket::ignite()
            .mount("/", routes![auto])
            .manage(counter)
            .attach(AdHoc::on_request("Check HEAD + Count", |req, _| {
                assert_eq!(req.method(), Method::Head);

                // This should be called exactly once.
                let c = req.guard::<State<Counter>>().unwrap();
                assert_eq!(c.0.fetch_add(1, Ordering::SeqCst), 0);
            }))
            .attach(AdHoc::on_response("Check GET", |req, res| {
                assert_eq!(req.method(), Method::Get);
                assert_eq!(res.body_string(), Some(RESPONSE_STRING.into()));
            }));

        let client = Client::new(rocket).unwrap();
        let mut response = client.head("/").dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert!(response.body().is_none());
    }
}
