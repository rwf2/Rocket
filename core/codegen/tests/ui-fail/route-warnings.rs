// must-compile-successfully

#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

// Check for unknown media types.

#[get("/", format = "application/x-custom")] //~ WARNING not a known media type
fn f0() {}

#[get("/", format = "x-custom/plain")] //~ WARNING not a known media type
fn f1() {}

#[get("/", format = "x-custom/x-custom")] //~ WARNING not a known media type
fn f2() {}

// Check if a data argument is used with a usually non-payload bearing method.

#[get("/", data = "<_foo>")] //~ WARNING used with non-payload-supporting method
fn g0(_foo: rocket::Data) {}

#[head("/", data = "<_foo>")] //~ WARNING used with non-payload-supporting method
fn g1(_foo: rocket::Data) {}

fn main() {  }
