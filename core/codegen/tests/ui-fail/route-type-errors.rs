#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

struct Q;

#[get("/<foo>")]
fn f0(foo: Q) {} //~ ERROR FromParam

#[get("/<foo..>")]
fn f1(foo: Q) {} //~ ERROR FromSegments

#[get("/?<foo>")]
fn f2(foo: Q) {} //~ ERROR FromFormValue

#[get("/?<foo..>")]
fn f3(foo: Q) {} //~ ERROR FromQuery

#[post("/", data = "<foo>")]
fn f4(foo: Q) {} //~ ERROR FromData

#[get("/<foo>")]
fn f5(a: Q, foo: Q) {}
//~^ ERROR FromParam
//~^^ ERROR FromRequest

#[get("/<foo>/other/<bar>/<good>/okay")]
fn f6(a: Q, foo: Q, good: usize, bar: Q) {}
//~^ ERROR FromParam
//~^^ ERROR FromParam
//~^^^ ERROR FromRequest

fn main() {  }
