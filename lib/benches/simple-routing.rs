#![feature(test, plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

use rocket::config::{Environment, Config};
use rocket::http::RawStr;

#[get("/")]
fn get_index() -> &'static str { "index" }

#[put("/")]
fn put_index() -> &'static str { "index" }

#[post("/")]
fn post_index() -> &'static str { "index" }

#[get("/a")]
fn index_a() -> &'static str { "index" }

#[get("/b")]
fn index_b() -> &'static str { "index" }

#[get("/c")]
fn index_c() -> &'static str { "index" }

#[get("/<a>")]
fn index_dyn_a(a: &RawStr) -> &'static str { "index" }

fn rocket() -> rocket::Rocket {
    let config = Config::new(Environment::Production).unwrap();
    rocket::custom(config, false)
        .mount("/", routes![get_index, put_index, post_index, index_a,
               index_b, index_c, index_dyn_a])
}

mod benches {
    extern crate test;

    use super::rocket;
    use self::test::Bencher;
    use rocket::testing::MockRequest;
    use rocket::http::Method::*;

    #[bench]
    fn bench_single_get_index(b: &mut Bencher) {
        let rocket = rocket();
        let mut request = MockRequest::new(Get, "/");

        b.iter(|| {
            request.dispatch_with(&rocket);
        });
    }

    #[bench]
    fn bench_get_put_post_index(b: &mut Bencher) {
        let rocket = rocket();

        // Hold all of the requests we're going to make during the benchmark.
        let mut requests = vec![];
        requests.push(MockRequest::new(Get, "/"));
        requests.push(MockRequest::new(Put, "/"));
        requests.push(MockRequest::new(Post, "/"));

        b.iter(|| {
            for request in requests.iter_mut() {
                request.dispatch_with(&rocket);
            }
        });
    }

    #[bench]
    fn bench_dynamic(b: &mut Bencher) {
        let rocket = rocket();

        // Hold all of the requests we're going to make during the benchmark.
        let mut requests = vec![];
        requests.push(MockRequest::new(Get, "/abc"));
        requests.push(MockRequest::new(Get, "/abcdefg"));
        requests.push(MockRequest::new(Get, "/123"));

        b.iter(|| {
            for request in requests.iter_mut() {
                request.dispatch_with(&rocket);
            }
        });
    }

    #[bench]
    fn bench_simple_routing(b: &mut Bencher) {
        let rocket = rocket();

        // Hold all of the requests we're going to make during the benchmark.
        let mut requests = vec![];
        for route in rocket.routes() {
            let request = MockRequest::new(route.method, route.path.path());
            requests.push(request);
        }

        // A few more for the dynamic route.
        requests.push(MockRequest::new(Get, "/abc"));
        requests.push(MockRequest::new(Get, "/abcdefg"));
        requests.push(MockRequest::new(Get, "/123"));

        b.iter(|| {
            for request in requests.iter_mut() {
                request.dispatch_with(&rocket);
            }
        });
    }
}
