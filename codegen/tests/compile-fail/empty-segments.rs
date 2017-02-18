#![feature(plugin)]
#![plugin(rocket_codegen)]

#[get("/a/b/c//d")] //~ ERROR paths cannot contain empty segments
fn get() -> &'static str { "hi" }

#[get("//")] //~ ERROR paths cannot contain empty segments
fn get1(name: &str) -> &'static str { "hi" }

#[get("/a/")] //~ ERROR paths cannot contain empty segments
fn get2(name: &str) -> &'static str { "hi" }

#[get("////")] //~ ERROR paths cannot contain empty segments
fn get3() -> &'static str { "hi" }

#[get("/a///")] //~ ERROR paths cannot contain empty segments
fn get4() -> &'static str { "hi" }

#[get("/a/b//")] //~ ERROR paths cannot contain empty segments
fn get5() -> &'static str { "hi" }

#[get("/a/b/c/")] //~ ERROR paths cannot contain empty segments
fn get6() -> &'static str { "hi" }

#[get("/a/b/c/d//e/")] //~ ERROR paths cannot contain empty segments
fn get7() -> &'static str { "hi" }

fn main() {  }
