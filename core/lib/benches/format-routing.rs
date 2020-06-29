#![feature(test)]
#![feature(proc_macro_hygiene)]

#[macro_use] extern crate rocket;

use rocket::config::{Environment, Config, LoggingLevel};

#[get("/", format = "application/json")]
fn get() -> &'static str { "get" }

#[post("/", format = "application/json")]
fn post() -> &'static str { "post" }

fn rocket() -> rocket::Rocket {
    let config = Config::build(Environment::Production).log_level(LoggingLevel::Off);
    rocket::custom(config.unwrap()).mount("/", routes![get, post])
}

#[allow(unused_must_use)]
mod benches {
    extern crate test;

    use super::rocket;
    use self::test::Bencher;
    use rocket::local::Client;
    use rocket::http::{Accept, ContentType};

    fn client(_rocket: rocket::Rocket) -> Option<Client> {
        unimplemented!("waiting for sync-client");
    }

    #[bench]
    fn accept_format(b: &mut Bencher) {
        let client = client(rocket()).unwrap();
        let mut request = client.get("/").header(Accept::JSON);
        b.iter(|| { request.mut_dispatch(); });
    }

    #[bench]
    fn wrong_accept_format(b: &mut Bencher) {
        let client = client(rocket()).unwrap();
        let mut request = client.get("/").header(Accept::HTML);
        b.iter(|| { request.mut_dispatch(); });
    }

    #[bench]
    fn content_type_format(b: &mut Bencher) {
        let client = client(rocket()).unwrap();
        let mut request = client.post("/").header(ContentType::JSON);
        b.iter(|| { request.mut_dispatch(); });
    }

    #[bench]
    fn wrong_content_type_format(b: &mut Bencher) {
        let client = client(rocket()).unwrap();
        let mut request = client.post("/").header(ContentType::Plain);
        b.iter(|| { request.mut_dispatch(); });
    }
}
