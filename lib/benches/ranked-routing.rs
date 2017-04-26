#![feature(test, plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

use rocket::config::{Environment, Config};
use rocket::http::RawStr;

#[get("/", format = "application/json")]
fn get() -> &'static str { "json" }

#[get("/", format = "text/html")]
fn get2() -> &'static str { "html" }

#[get("/", format = "text/plain")]
fn get3() -> &'static str { "plain" }

#[post("/", format = "application/json")]
fn post() -> &'static str { "json" }

#[post("/", format = "text/html")]
fn post2() -> &'static str { "html" }

#[post("/", format = "text/plain")]
fn post3() -> &'static str { "plain" }

fn rocket() -> rocket::Rocket {
    let config = Config::new(Environment::Production).unwrap();
    rocket::custom(config, false)
        .mount("/", routes![get, get2, get3])
        .mount("/", routes![post, post2, post3])
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
        let mut requests = vec![];
        requests.push(MockRequest::new(Get, "/").header(Accept::JSON));
        requests.push(MockRequest::new(Get, "/").header(Accept::HTML));
        requests.push(MockRequest::new(Get, "/").header(Accept::Plain));

        b.iter(|| {
            for request in requests.iter_mut() {
                request.dispatch_with(&rocket);
            }
        });
    }

    #[bench]
    fn content_type_format(b: &mut Bencher) {
        let rocket = rocket();
        let mut requests = vec![];
        requests.push(MockRequest::new(Post, "/").header(ContentType::JSON));
        requests.push(MockRequest::new(Post, "/").header(ContentType::HTML));
        requests.push(MockRequest::new(Post, "/").header(ContentType::Plain));

        b.iter(|| {
            for request in requests.iter_mut() {
                request.dispatch_with(&rocket);
            }
        });
    }
}
