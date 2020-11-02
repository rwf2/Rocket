#[macro_use] extern crate rocket;
#[macro_use] extern crate bencher;

use rocket::local::blocking::Client;

#[get("/", format = "application/json")]
fn get() -> &'static str { "get" }

#[post("/", format = "application/json")]
fn post() -> &'static str { "post" }

fn rocket() -> rocket::Rocket {
    rocket::custom(rocket::Config::figment().merge(("log_level", "off")))
        .mount("/", routes![get, post])
}

use bencher::Bencher;
use rocket::http::{Accept, ContentType};

fn accept_format(b: &mut Bencher) {
    let client = Client::tracked(rocket()).unwrap();
    let request = client.get("/").header(Accept::JSON);
    b.iter(|| { request.clone().dispatch(); });
}

fn wrong_accept_format(b: &mut Bencher) {
    let client = Client::tracked(rocket()).unwrap();
    let request = client.get("/").header(Accept::HTML);
    b.iter(|| { request.clone().dispatch(); });
}

fn content_type_format(b: &mut Bencher) {
    let client = Client::tracked(rocket()).unwrap();
    let request = client.post("/").header(ContentType::JSON);
    b.iter(|| { request.clone().dispatch(); });
}

fn wrong_content_type_format(b: &mut Bencher) {
    let client = Client::tracked(rocket()).unwrap();
    let request = client.post("/").header(ContentType::Plain);
    b.iter(|| { request.clone().dispatch(); });
}

benchmark_main!(benches);
benchmark_group! {
    benches,
    accept_format,
    wrong_accept_format,
    content_type_format,
    wrong_content_type_format,
}
