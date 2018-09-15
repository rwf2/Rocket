#![feature(plugin, decl_macro, proc_macro_non_items)]
#![plugin(rocket_codegen)]

#[macro_use] extern crate rocket;

mod common;

#[test]
fn test_production_config() {
    common::test_config(rocket::config::Environment::Production);
}
