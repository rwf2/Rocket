#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

use rocket::{Error, Request};

#[catch(404)]
fn err_a(_a: Error, _b: Request, _c: Error) -> &'static str { "hi" }

#[catch(404)]
fn err_b(_a: (isize, usize)) -> &'static str { "hi" }
