#![allow(dead_code)] // This test is only here so that we can ensure it compiles.

#[macro_use] extern crate rocket;

use rocket::{Request, State};
use rocket::response::{Responder, Result};

struct SomeState;

pub struct CustomResponder<'r, R> {
    responder: R,
    state: &'r SomeState,
}

impl<'r, 'o: 'r, R: Responder<'r, 'o>> Responder<'r, 'o> for CustomResponder<'r, R> {
    fn respond_to(self, req: &'r Request<'_>) -> Result<'o> {
        self.responder.respond_to(req)
    }
}

#[get("/unit_state")]
fn unit_state(state: &State<SomeState>) -> CustomResponder<()> {
    CustomResponder { responder: (), state: &*state }
}

#[get("/string_state")]
fn string_state(state: &State<SomeState>) -> CustomResponder<String> {
    CustomResponder { responder: "".to_string(), state: &*state }
}
