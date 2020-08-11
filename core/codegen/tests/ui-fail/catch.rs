#[macro_use] extern crate rocket;

use rocket::Request;

#[catch(404)]
struct Catcher(String);

#[catch(404)]
const CATCH: &str = "Catcher";

#[catch("404")]
fn e1(_request: &Request) { }

#[catch(code = "404")]
fn e2(_request: &Request) { }

#[catch(code = 404)]
fn e3(_request: &Request) { }

#[catch(99)]
fn e4(_request: &Request) { }

#[catch(600)]
fn e5(_request: &Request) { }

#[catch(400, message = "foo")]
fn e5(_request: &Request) { }

#[catch(404)]
fn f3(_request: &Request, other: bool) { }

fn main() { }
