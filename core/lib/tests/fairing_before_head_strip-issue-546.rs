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
    use std::io::Cursor;

    use rocket::fairing::AdHoc;
    use rocket::local::blocking::Client;
    use rocket::http::{Method, Status};

    #[test]
    fn not_auto_handled() {
        let rocket = rocket::build()
            .mount("/", routes![head])
            .attach(AdHoc::on_request("Check HEAD", |req, _| {
                Box::pin(async move {
                    assert_eq!(req.method(), Method::Head);
                })
            }))
            .attach(AdHoc::on_response("Check HEAD 2", |req, res| {
                Box::pin(async move {
                    assert_eq!(req.method(), Method::Head);
                    let body_bytes = res.body_mut().to_bytes().await.unwrap();
                    assert_eq!(body_bytes, RESPONSE_STRING.as_bytes());
                    res.set_sized_body(body_bytes.len(), Cursor::new(body_bytes));
                })
            }));

        let client = Client::debug(rocket).unwrap();
        let response = client.head("/").dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert!(response.into_string().unwrap_or_default().is_empty());
    }

    #[test]
    fn auto_handled() {
        #[derive(Default)]
        struct Counter(AtomicUsize);

        let counter = Counter::default();
        let rocket = rocket::build()
            .mount("/", routes![auto])
            .manage(counter)
            .attach(AdHoc::on_request("Check HEAD + Count", |req, _| {
                Box::pin(async move {
                    assert_eq!(req.method(), Method::Head);

                    // This should be called exactly once.
                    let c = req.rocket().state::<Counter>().unwrap();
                    assert_eq!(c.0.fetch_add(1, Ordering::SeqCst), 0);
                })
            }))
            .attach(AdHoc::on_response("Check GET", |req, res| {
                Box::pin(async move {
                    assert_eq!(req.method(), Method::Get);
                    let body_bytes = res.body_mut().to_bytes().await.unwrap();
                    assert_eq!(body_bytes, RESPONSE_STRING.as_bytes());
                    res.set_sized_body(body_bytes.len(), Cursor::new(body_bytes));
                })
            }));

        let client = Client::debug(rocket).unwrap();
        let response = client.head("/").dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert!(response.into_string().unwrap_or_default().is_empty());
    }
}

#[test]
fn fairing_before_status_body_strip() {
    use std::io::Cursor;
    use rocket::fairing::AdHoc;
    use rocket::http::Status;
    use rocket::local::blocking::Client;

    for status in [Status::NoContent, Status::NotModified] {
        let app = rocket::build().mount("/", routes![auto])
            .attach(AdHoc::on_response("Set Status", move |_, response| Box::pin(async move {
                let bytes = response.body_mut().to_bytes().await.unwrap();
                assert_eq!(bytes, RESPONSE_STRING.as_bytes());
                response.set_status(status);
                response.set_sized_body(bytes.len(), Cursor::new(bytes));
                response.set_raw_header("Content-Length", "123");
            })));

        let client = Client::debug(app).unwrap();
        let response = client.get("/").dispatch();
        let length = response.headers().get_one("Content-Length");
        if status == Status::NoContent {
            assert_eq!(length, None);
            assert_eq!(response.body().preset_size(), None);
        } else {
            assert_eq!(length, Some("123"));
            assert_eq!(response.body().preset_size(), Some(RESPONSE_STRING.len()));
        }

        assert!(response.into_bytes().unwrap_or_default().is_empty());
    }
}
