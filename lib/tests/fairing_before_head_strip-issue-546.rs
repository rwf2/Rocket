#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate rocket;

const RESPONSE_STRING: &'static str = "{'test': 'dont strip before fairing' }";

#[head("/")]
fn index() -> &'static str {
    RESPONSE_STRING
}

#[get("/")]
fn auto() -> &'static str {
    RESPONSE_STRING
}

mod fairing_before_head_strip {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use rocket::fairing::AdHoc;
    use rocket::http::Method;
    use rocket::local::Client;
    use rocket::http::Status;
    use rocket::State;

    #[derive(Default)]
    struct Counter {
        get: AtomicUsize,
    }

    #[test]
    fn not_empty_before_fairing() {
        let rocket = rocket::ignite()
            .mount("/", routes![index])
            .attach(AdHoc::on_response(|req, res| {
                assert_eq!(req.method(), Method::Head);
                assert_eq!(res.body_string(), Some(RESPONSE_STRING.into()));
            }));

        let client = Client::new(rocket).unwrap();
        let response = client.head("/").dispatch();
        assert_eq!(response.status(), Status::Ok);
    }

    #[test]
    fn not_empty_before_fairing_autohandeled() {
        let counter = Counter::default();
        let rocket = rocket::ignite()
            .mount("/", routes![auto])
            .manage(counter)
            .attach(AdHoc::on_request(|req, _| {
                 let c = req.guard::<State<Counter>>().unwrap();

                 assert_eq!(c.get.fetch_add(1, Ordering::Release), 0);
            }))
            .attach(AdHoc::on_response(|req, res| {
                assert_eq!(req.method(), Method::Get);
                assert_eq!(res.body_string(), Some(RESPONSE_STRING.into()));
            }));

        let client = Client::new(rocket).unwrap();
        let response = client.head("/").dispatch();
        assert_eq!(response.status(), Status::Ok);
    }
}
