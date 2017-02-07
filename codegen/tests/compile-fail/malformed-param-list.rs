#![feature(plugin)]
#![plugin(rocket_codegen)]

#[get("/><")] //~ ERROR malformed
fn get() -> &'static str { "hi" }

#[get("/<name><")] //~ ERROR malformed
fn get1(name: &str) -> &'static str { "hi" }

#[get("/<<<<name><")] //~ ERROR malformed
fn get2(name: &str) -> &'static str { "hi" }

#[get("/<!>")] //~ ERROR identifiers
fn get3() -> &'static str { "hi" }

#[get("/<_>")] //~ ERROR ignored
fn get4() -> &'static str { "hi" }

#[get("/<1>")] //~ ERROR identifiers
fn get5() -> &'static str { "hi" }

#[get("/<>name><")] //~ ERROR malformed
fn get6() -> &'static str { "hi" }

#[get("/<name>:<id>")] //~ ERROR identifiers
fn get7() -> &'static str { "hi" }

#[get("/<>")] //~ ERROR empty
fn get8() -> &'static str { "hi" }

fn main() {  }
