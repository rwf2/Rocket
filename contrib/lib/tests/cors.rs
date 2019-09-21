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
    use rocket_contrib::cors::*;
    use rocket::local::*;
    use rocket::http::Status;

    #[get("/test")]
    pub fn sample_get_route() -> &'static str {
        "Hi"
    }

    #[delete("/test")]
    pub fn sample_delete_route() -> &'static str {
        "Hi"
    }

    // TODO name after Cross Origin.  Example CrossOrigin
    
    // Include on all responses 
    //   Access-Control-Allow-Credentials
    //   Access-Control-Allow-Origin
    // Requested on preflight
    //   `Access-Control-Request-Method` 
    //   `Access-Control-Request-Headers` 
    // Include on preflight
    //   Access-Control-Allow-Methods` 
    //   `Access-Control-Allow-Headers` 
    //   `Access-Control-Max-Age` 
    //   `Access-Control-Expose-Headers` 

    #[test]
    pub fn test_one_method() {
        let rocket = rocket::ignite()
            .mount("/", routes![sample_delete_route])
            .attach(CorsFairingBuilder::new()
                .build());

        let client = Client::new(rocket).expect("valid rocket instance");
        let response = client.options("/test").dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.headers().get_one("Access-Control-Allow-Methods"), Some("DELETE".into()));
    }

    // Test to ensure the method names are collected when multiple
    #[test]
    pub fn test_many_method() {
        let rocket = rocket::ignite()
            .mount("/", routes![sample_get_route, sample_delete_route])
            .attach(CorsFairingBuilder::new()
                .build());

        let client = Client::new(rocket).expect("valid rocket instance");

        let response = client.options("/test").dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.headers().get_one("Access-Control-Allow-Methods"), Some("DELETE, GET".into()));
    }
}
