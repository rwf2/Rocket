#![feature(plugin, decl_macro, proc_macro_hygiene)]
#![plugin(rocket_codegen)]

#[macro_use] extern crate rocket;

mod common;

#[test]
fn test_development_config() {
    common::test_config(rocket::config::Environment::Development);
}
