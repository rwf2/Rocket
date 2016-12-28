#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
use rocket::config::Environment;

mod common;

#[test]
fn test() {
    common::test_config(Environment::Staging);
    common::test_hello();
}
