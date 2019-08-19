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

    #[test]
    pub fn test_basic() {
        let _ = CorsFairingBuilder::new();
    }

    #[get("/test")]
    pub fn test_get_route() -> &'static str {
        "Hi"
    }

    #[test]
    pub fn test_one_method() {
        let rocket = rocket::ignite()
            .mount("/", routes![test_get_route])
            .attach(CorsFairingBuilder::new()
                .build());

        let mut count = 0;
        for _ in rocket.routes() {
            count = count + 1;
        }
        assert_eq!(2, count);

    }

    // Test to ensure the method names are collected when multiple
    #[test]
    pub fn test_many_method() {

    }
}
