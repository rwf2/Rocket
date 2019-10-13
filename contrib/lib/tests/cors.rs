#![feature(proc_macro_hygiene)]
#[macro_use] extern crate rocket;

#[cfg(feature = "cors")]
mod cors_tests {
    use rocket_contrib::cors::CorsFairing;
    use rocket_contrib::cors::config::*;
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



    #[test]
    pub fn test_one_method() {
        let rocket = rocket::ignite()
            .mount("/", routes![sample_delete_route])
            .attach(CorsFairing::with_config(CorsFairingConfig::with_any_origin()));

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
            .attach(CorsFairing::with_config(CorsFairingConfig::with_any_origin()));


        let client = Client::new(rocket).expect("valid rocket instance");

        let response = client.options("/test").dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.headers().get_one("Access-Control-Allow-Methods"), Some("DELETE, GET".into()));
    }

    /// Test that the any origin works correctly.
    #[test]
    pub fn test_any_origin() {
        let rocket = rocket::ignite()
            .mount("/", routes![sample_delete_route])
            .attach(CorsFairing::with_config(CorsFairingConfig::with_any_origin()));


        let client = Client::new(rocket).expect("valid rocket instance");

        let response = client.options("/test").dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.headers().get_one("Access-Control-Allow-Origin"), Some("*".into()));
    }

    #[test]
    pub fn test_explicit_origin() {
        let rocket = rocket::ignite()
            .mount("/", routes![sample_delete_route])
            .attach(CorsFairing::with_config(CorsFairingConfig::with_origin("https://example.com/".to_owned())));


        let client = Client::new(rocket).expect("valid rocket instance");

        let response = client.options("/test").dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.headers().get_one("Access-Control-Allow-Origin"), Some("https://example.com/".into()));
    }
}
