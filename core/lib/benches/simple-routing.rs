#![feature(test)]
#![feature(proc_macro_hygiene)]

#[macro_use] extern crate rocket;

use rocket::config::{Environment, Config, LoggingLevel};
use rocket::http::RawStr;

#[get("/")]
fn hello_world() -> &'static str { "Hello, world!" }

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

#[get("/<_a>")]
fn index_dyn_a(_a: &RawStr) -> &'static str { "index" }

fn hello_world_rocket() -> rocket::Rocket {
    let config = Config::build(Environment::Production).log_level(LoggingLevel::Off);
    rocket::custom(config.unwrap()).mount("/", routes![hello_world])
}

fn rocket() -> rocket::Rocket {
    let config = Config::build(Environment::Production).log_level(LoggingLevel::Off);
    rocket::custom(config.unwrap())
        .mount("/", routes![get_index, put_index, post_index, index_a,
               index_b, index_c, index_dyn_a])
}

#[allow(unused_must_use)]
mod benches {
    extern crate test;

    use super::{hello_world_rocket, rocket};
    use self::test::Bencher;
    use rocket::local::blocking::Client;

    fn client(_rocket: rocket::Rocket) -> Option<Client> {
        unimplemented!("waiting for sync-client");
    }

    #[bench]
    fn bench_hello_world(b: &mut Bencher) {
        let client = client(hello_world_rocket()).unwrap();

        b.iter(|| {
            client.get("/").dispatch();
        });
    }

    #[bench]
    fn bench_single_get_index(b: &mut Bencher) {
        let client = client(rocket()).unwrap();

        b.iter(|| {
            client.get("/").dispatch();
        });
    }

    #[bench]
    fn bench_get_put_post_index(b: &mut Bencher) {
        let client = client(rocket()).unwrap();

        // Hold all of the requests we're going to make during the benchmark.
        let mut requests = vec![];
        requests.push(client.get("/"));
        requests.push(client.put("/"));
        requests.push(client.post("/"));

        b.iter(|| {
            for request in &requests {
                request.clone().dispatch();
            }
        });
    }

    #[bench]
    fn bench_dynamic(b: &mut Bencher) {
        let client = client(rocket()).unwrap();

        // Hold all of the requests we're going to make during the benchmark.
        let mut requests = vec![];
        requests.push(client.get("/abc"));
        requests.push(client.get("/abcdefg"));
        requests.push(client.get("/123"));

        b.iter(|| {
            for request in &requests {
                request.clone().dispatch();
            }
        });
    }

    #[bench]
    fn bench_simple_routing(b: &mut Bencher) {
        let client = client(rocket()).unwrap();

        // Hold all of the requests we're going to make during the benchmark.
        let mut requests = vec![];
        for route in client.cargo().routes() {
            let request = client.req(route.method, route.uri.path());
            requests.push(request);
        }

        // A few more for the dynamic route.
        requests.push(client.get("/abc"));
        requests.push(client.get("/abcdefg"));
        requests.push(client.get("/123"));

        b.iter(|| {
            for request in &requests {
                request.clone().dispatch();
            }
        });
    }
}
