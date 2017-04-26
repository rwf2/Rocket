#![feature(test, plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

use rocket::config::{Environment, Config};
use rocket::http::RawStr;

#[get("/", format = "application/json")]
fn get() -> &'static str { "get" }

#[post("/", format = "application/json")]
fn post() -> &'static str { "post" }

fn rocket() -> rocket::Rocket {
    let config = Config::new(Environment::Production).unwrap();
    rocket::custom(config, false)
        .mount("/", routes![get, post])
}

mod benches {
    extern crate test;

    use super::rocket;
    use self::test::Bencher;
    use rocket::testing::MockRequest;
    use rocket::http::Method::*;
    use rocket::http::{Accept, ContentType};

    #[bench]
    fn accept_format(b: &mut Bencher) {
        let rocket = rocket();
        let mut request = MockRequest::new(Get, "/").header(Accept::JSON);
        b.iter(|| { request.dispatch_with(&rocket); });
    }

    #[bench]
    fn wrong_accept_format(b: &mut Bencher) {
        let rocket = rocket();
        let mut request = MockRequest::new(Get, "/").header(Accept::HTML);
        b.iter(|| { request.dispatch_with(&rocket); });
    }

    #[bench]
    fn content_type_format(b: &mut Bencher) {
        let rocket = rocket();
        let mut request = MockRequest::new(Post, "/").header(ContentType::JSON);
        b.iter(|| { request.dispatch_with(&rocket); });
    }

    #[bench]
    fn wrong_content_type_format(b: &mut Bencher) {
        let rocket = rocket();
        let mut request = MockRequest::new(Post, "/").header(ContentType::Plain);
        b.iter(|| { request.dispatch_with(&rocket); });
    }
}
