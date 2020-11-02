#[macro_use] extern crate rocket;
#[macro_use] extern crate bencher;

use rocket::http::RawStr;

#[get("/")]
fn hello_world() -> &'static str { "Hello, world!" }

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
    let config = rocket::Config::figment().merge(("log_level", "off"));
    rocket::custom(config).mount("/", routes![hello_world])
}

fn rocket() -> rocket::Rocket {
    hello_world_rocket()
        .mount("/", routes![
            put_index, post_index, index_a, index_b, index_c, index_dyn_a
        ])
}

use bencher::Bencher;
use rocket::local::blocking::Client;

fn bench_hello_world(b: &mut Bencher) {
    let client = Client::tracked(hello_world_rocket()).unwrap();

    b.iter(|| {
        client.get("/").dispatch();
    });
}

fn bench_single_get_index(b: &mut Bencher) {
    let client = Client::tracked(rocket()).unwrap();

    b.iter(|| {
        client.get("/").dispatch();
    });
}

fn bench_get_put_post_index(b: &mut Bencher) {
    let client = Client::tracked(rocket()).unwrap();

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

fn bench_dynamic(b: &mut Bencher) {
    let client = Client::tracked(rocket()).unwrap();

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

fn bench_simple_routing(b: &mut Bencher) {
    let client = Client::tracked(rocket()).unwrap();

    // Hold all of the requests we're going to make during the benchmark.
    let mut requests = vec![];
    for route in client.rocket().routes() {
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

benchmark_main!(benches);
benchmark_group! {
    benches,
    bench_hello_world,
    bench_single_get_index,
    bench_get_put_post_index,
    bench_dynamic,
    bench_simple_routing,
}
