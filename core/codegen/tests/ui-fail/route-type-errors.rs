#[macro_use] extern crate rocket;

struct Q;

#[get("/<foo>")]
fn f0(foo: Q) {}

#[get("/<foo..>")]
fn f1(foo: Q) {}

#[get("/?<foo>")]
fn f2(foo: Q) {}

#[get("/?<foo..>")]
fn f3(foo: Q) {}

#[post("/", data = "<foo>")]
fn f4(foo: Q) {}

#[get("/<foo>")]
fn f5(a: Q, foo: Q) {}

#[get("/<foo>/other/<bar>/<good>/okay")]
fn f6(a: Q, foo: Q, good: usize, bar: Q) {}

fn main() {  }
