//! Example forces various warnings
//! don't do this stuff ;)
#![feature(decl_macro)]
#![feature(plugin)]
#![plugin(rocket_codegen)]
extern crate rocket;
extern crate rocket_contrib;

use rocket::config::{Config, Environment};

use rocket_contrib::space_helmet::*;

#[get("/")]
fn index() -> &'static str {
    println!("hello");
        "Hello, world!"
}

fn rocket() -> rocket::Rocket {
    let helmet = Helmet::new();
    //Using tls without setting hsts is not best practice, try this instead.
    //let helmet = Helmet::new().hsts(HSTSPolicy::default());

    let mut config = Config::build(Environment::Production).unwrap();
    config.set_tls("private/cert.pem", "private/key.pem").expect("tls set failed");

    let rocket = rocket::custom(config)
                    .mount("/", routes![index])
                    .attach(helmet)
                    .attach(Helmet::new()); //this will cause warnings as helmet will try to repeatedly
                                            //apply the same headers.
    rocket
}

fn main() {
    rocket().launch();
}
