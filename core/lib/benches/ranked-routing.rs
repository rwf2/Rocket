#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

use rocket::config::{Environment, Config, LoggingLevel};

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
    let config = Config::build(Environment::Production).log_level(LoggingLevel::Off);
    rocket::custom(config.unwrap())
        .mount("/", routes![get, get2, get3])
        .mount("/", routes![post, post2, post3])
}

mod benches {
    extern crate test;

    use super::rocket;
    use self::test::Bencher;
    use rocket::local::Client;
    use rocket::http::{Accept, ContentType};

    #[bench]
    fn accept_format(b: &mut Bencher) {
        let client = Client::new(rocket()).unwrap();
        let mut requests = vec![];
        requests.push(client.get("/").header(Accept::JSON));
        requests.push(client.get("/").header(Accept::HTML));
        requests.push(client.get("/").header(Accept::Plain));

        b.iter(|| {
            for request in requests.iter_mut() {
                request.mut_dispatch();
            }
        });
    }

    #[bench]
    fn content_type_format(b: &mut Bencher) {
        let client = Client::new(rocket()).unwrap();
        let mut requests = vec![];
        requests.push(client.post("/").header(ContentType::JSON));
        requests.push(client.post("/").header(ContentType::HTML));
        requests.push(client.post("/").header(ContentType::Plain));

        b.iter(|| {
            for request in requests.iter_mut() {
                request.mut_dispatch();
            }
        });
    }
}
