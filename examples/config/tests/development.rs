#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

mod common;

#[test]
fn test_development_config() {
    common::test_config(rocket::config::Environment::Development);
}
