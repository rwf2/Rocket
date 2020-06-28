#![feature(test)]
#![feature(proc_macro_hygiene)]

#[macro_use] extern crate rocket;

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

#[allow(unused_must_use)]
mod benches {
    extern crate test;

    use super::rocket;
    use self::test::Bencher;
    use rocket::local::blocking::Client;
    use rocket::http::{Accept, ContentType};

    fn client(_rocket: rocket::Rocket) -> Option<Client> {
        unimplemented!("waiting for sync-client");
    }

    #[bench]
    fn accept_format(b: &mut Bencher) {
        let client = client(rocket()).unwrap();
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

    #[bench]
    fn content_type_format(b: &mut Bencher) {
        let client = client(rocket()).unwrap();
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
}
