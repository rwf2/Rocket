#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate rocket;

#[route("GET", "/hello")]
fn get1() -> &'static str { "hi" }

#[route(120, "/hello")]
fn get2() -> &'static str { "hi" }

#[route("CONNECT", "/hello")]
fn get3() -> &'static str { "hi" }
