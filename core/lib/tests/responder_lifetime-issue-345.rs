#![feature(proc_macro_hygiene, decl_macro)]
#![allow(dead_code)] // This test is only here so that we can ensure it compiles.

#[macro_use] extern crate rocket;

use rocket::State;
use rocket::response::{self, Responder};

struct SomeState;

pub struct CustomResponder<'r, R> {
    responder: R,
    state: &'r SomeState,
}

impl<'r, R: Responder<'r>> Responder<'r> for CustomResponder<'r, R> {
    fn respond_to(self, _: &rocket::Request) -> response::Result<'r> {
        unimplemented!()
    }
}

#[get("/unit_state")]
fn unit_state(state: State<SomeState>) -> CustomResponder<()> {
    CustomResponder { responder: (), state: state.inner() }
}

#[get("/string_state")]
fn string_state(state: State<SomeState>) -> CustomResponder<String> {
    CustomResponder { responder: "".to_string(), state: state.inner() }
}
