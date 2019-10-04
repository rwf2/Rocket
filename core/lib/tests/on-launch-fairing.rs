#![feature(proc_macro_hygiene)]

#[macro_use] extern crate rocket;

use std::sync::atomic::{AtomicBool, Ordering};

use rocket::{
    Data,
    fairing::{
        Fairing,
        Kind,
        Info
    },
    Request,
    Rocket
};

struct TestFairing {
    initialized: AtomicBool
}

impl TestFairing {
    pub fn new() -> Self {
        Self {
            initialized: AtomicBool::new(false)
        }
    }
}

impl Fairing for TestFairing
{
    fn info(&self) -> Info
    {
        Info {
            name: "TestFairing for on_launch callback",
            kind: Kind::Launch | Kind::Request
        }
    }

    fn on_launch(&self, _rocket: &Rocket)
    {
        self.initialized.store(true, Ordering::Relaxed);
    }

    fn on_request(&self, _request: &mut Request<'_>, _data: &Data)
    {
        if self.initialized.load(Ordering::Relaxed) == false {
            panic!("This should have been set to true in `Fairing::on_launch`");
        }
    }
}

#[get("/<name>/<age>")]
fn hello(name: String, age: u8) -> String {
    format!("Hello, {} year old named {}!", age, name)
}

mod on_launch_fairing {
    use super::*;
    use rocket::local::Client;

    #[test]
    fn test_on_launch() {
        let rocket = rocket::ignite()
            .attach(TestFairing::new())
            .mount("/hello", routes![hello]);
        let client = Client::new(rocket).expect("valid rocket instance");

        let req = client.get("/hello/John%20Doe/37");
        let mut response = req.dispatch();
        let _body = response.body_string();

        // At this point, if the test succeed, no panic happened
    }
}