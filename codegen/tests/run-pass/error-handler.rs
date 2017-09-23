#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

use rocket::{Error, Request};

#[catch(404)]
fn err0() -> &'static str { "hi" }

#[catch(404)]
fn err1a(_err: Error) -> &'static str { "hi" }

#[catch(404)]
fn err1b(_req: &Request) -> &'static str { "hi" }

#[catch(404)]
fn err2a(_err: Error, _req: &Request) -> &'static str { "hi" }

#[catch(404)]
fn err2b<'a>(_err: Error, _req: &'a Request) -> &'a str { "hi" }

fn main() {
}

