#[macro_use] extern crate rocket;
#[macro_use] extern crate bencher;

use rocket::config::{Environment, Config, LoggingLevel};

#[get("/", format = "application/json", rank = 1)]
fn get() -> &'static str { "json" }

#[get("/", format = "text/html", rank = 2)]
fn get2() -> &'static str { "html" }

#[get("/", format = "text/plain", rank = 3)]
fn get3() -> &'static str { "plain" }

#[post("/", format = "application/json")]
fn post() -> &'static str { "json" }

#[post("/", format = "text/html")]
fn post2() -> &'static str { "html" }

#[post("/", format = "text/plain")]
fn post3() -> &'static str { "plain" }

fn rocket() -> rocket::Rocket {
    let config = Config::build(Environment::Production).log_level(LoggingLevel::Off);
    rocket::custom(config.unwrap())
        .mount("/", routes![get, get2, get3])
        .mount("/", routes![post, post2, post3])
}

use bencher::Bencher;
use rocket::local::blocking::Client;
use rocket::http::{Accept, ContentType};

fn accept_format(b: &mut Bencher) {
    let client = Client::new(rocket()).unwrap();
    let requests = vec![
        client.get("/").header(Accept::JSON),
        client.get("/").header(Accept::HTML),
        client.get("/").header(Accept::Plain),
    ];

    b.iter(|| {
        for request in &requests {
            request.clone().dispatch();
        }
    });
}

fn content_type_format(b: &mut Bencher) {
    let client = Client::new(rocket()).unwrap();
    let requests = vec![
        client.post("/").header(ContentType::JSON),
        client.post("/").header(ContentType::HTML),
        client.post("/").header(ContentType::Plain),
    ];

    b.iter(|| {
        for request in &requests {
            request.clone().dispatch();
        }
    });
}

benchmark_main!(benches);
benchmark_group! {
    benches,
    accept_format,
    content_type_format,
}
