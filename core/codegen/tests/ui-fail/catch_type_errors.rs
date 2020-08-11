#[macro_use] extern crate rocket;

use rocket::Request;

#[catch(404)]
fn f1(_request: &Request) -> usize {
    10
}

#[catch(404)]
fn f2(_request: &Request) -> bool {
    false
}

#[catch(404)]
fn f3(_request: bool) -> usize {
    10
}

#[catch(404)]
fn f4() -> usize {
    10
}

fn main() {  }
