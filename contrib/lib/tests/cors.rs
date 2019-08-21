#![feature(proc_macro_hygiene)]
#[macro_use] extern crate rocket;

// TODO Test with an allow-any origin
// TODO Test with two origins
// TODO Test with one origin
// TODO Test a variety of methods (delete, put, post)
// TODO Test with headers
// TODO Test with no headers
// TODO Test with multiple headers
// note, I think all of these can be integration tests
#[cfg(feature = "cors")]
mod cors_tests {
    use rocket::http::Method;
    use rocket::Route;
    use rocket_contrib::cors::*;
    use rocket::local::*;
    use rocket::http::Status;

    #[test]
    pub fn test_basic() {
        let _ = CorsFairingBuilder::new();
    }

    #[get("/test")]
    pub fn sample_get_route() -> &'static str {
        "Hi"
    }

    #[delete("/test")]
    pub fn sample_delete_route() -> &'static str {
        "Hi"
    }

    #[test]
    pub fn test_one_method() {
        let rocket = rocket::ignite()
            .mount("/", routes![sample_get_route, sample_delete_route])
            .attach(CorsFairingBuilder::new()
                .build());

        let routes : Vec<&Route> = rocket.routes()
            .filter(|x| x.method == Method::Options)
            .filter(|x| x.uri.path() == "/test")
            .collect();
        assert_eq!(1, routes.len());

        let client = Client::new(rocket).expect("valid rocket instance");

        // Dispatch a request to 'GET /' and validate the response.
        let response = client.options("/test").dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.headers().get_one("Access-Control-Allow-Methods"), Some("DELETE, GET".into()));
    }

    // Test to ensure the method names are collected when multiple
    #[test]
    pub fn test_many_method() {

    }
}
