#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

#[get("/", format = "applicationx-custom")] //~ ERROR malformed
//~^ ERROR `format` must be a "media/type"
fn one() -> &'static str { "hi" }

#[get("/", format = "")] //~ ERROR malformed
//~^ ERROR `format` must be a "media/type"
fn two() -> &'static str { "hi" }

#[get("/", format = "//")] //~ ERROR malformed
//~^ ERROR `format` must be a "media/type"
fn three() -> &'static str { "hi" }

#[get("/", format = "/")] //~ ERROR malformed
//~^ ERROR `format` must be a "media/type"
fn four() -> &'static str { "hi" }

#[get("/", format = "a/")] //~ ERROR malformed
//~^ ERROR `format` must be a "media/type"
fn five() -> &'static str { "hi" }

#[get("/", format = "/a")] //~ ERROR malformed
//~^ ERROR `format` must be a "media/type"
fn six() -> &'static str { "hi" }

#[get("/", format = "/a/")] //~ ERROR malformed
//~^ ERROR `format` must be a "media/type"
fn seven() -> &'static str { "hi" }

#[get("/", format = "a/b/")] //~ ERROR malformed
//~^ ERROR `format` must be a "media/type"
fn eight() -> &'static str { "hi" }

fn main() {  }
