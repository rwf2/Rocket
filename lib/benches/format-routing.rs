#![feature(test, plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

use rocket::config::{Environment, Config};

#[get("/", format = "application/json")]
fn get() -> &'static str { "get" }

#[post("/", format = "application/json")]
fn post() -> &'static str { "post" }

fn rocket() -> rocket::Rocket {
    let config = Config::new(Environment::Production).unwrap();
    rocket::custom(config, false).mount("/", routes![get, post])
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
        let mut request = client.get("/").header(Accept::JSON);
        b.iter(|| { request.mut_dispatch(); });
    }

    #[bench]
    fn wrong_accept_format(b: &mut Bencher) {
        let client = Client::new(rocket()).unwrap();
        let mut request = client.get("/").header(Accept::HTML);
        b.iter(|| { request.mut_dispatch(); });
    }

    #[bench]
    fn content_type_format(b: &mut Bencher) {
        let client = Client::new(rocket()).unwrap();
        let mut request = client.post("/").header(ContentType::JSON);
        b.iter(|| { request.mut_dispatch(); });
    }

    #[bench]
    fn wrong_content_type_format(b: &mut Bencher) {
        let client = Client::new(rocket()).unwrap();
        let mut request = client.post("/").header(ContentType::Plain);
        b.iter(|| { request.mut_dispatch(); });
    }
}
