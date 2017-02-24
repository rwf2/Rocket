#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

use rocket::config::{Config, Environment};
use rocket::config::ConfigError;
use rocket::logger::LoggingLevel;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

fn main() {
    try_config().unwrap()
}

#[allow(dead_code)]
fn try_config() -> Result<(), ConfigError> {
    let config = Config::build(Environment::Development)
        .address("127.0.0.1")
        .port(0) // 0 will request that the OS assigns a port.
        .log_level(LoggingLevel::Debug)
        .finalize()?;
    let logging = true;
    let on_success = |addr| {
            println!("Resulting address: {}", addr);
    };
    rocket::custom(config, logging)
        .mount("/", routes![index])
        .launch(on_success);
    Ok(())
}
