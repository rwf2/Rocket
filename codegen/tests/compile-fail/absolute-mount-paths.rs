#![feature(plugin)]
#![plugin(rocket_codegen)]

#[get("a")] //~ ERROR absolute
fn get() -> &'static str { "hi" }

#[get("")] //~ ERROR absolute
fn get1(id: usize) -> &'static str { "hi" }

#[get("a/b/c")] //~ ERROR absolute
fn get2(id: usize) -> &'static str { "hi" }

fn main() {  }
