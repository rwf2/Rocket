#[macro_use] extern crate rocket;

use rocket::Request;

#[catch(404)]
fn f1(_request: &Request) -> usize {
    //~^ ERROR usize: rocket::response::Responder
    10
}

#[catch(404)]
fn f2(_request: &Request) -> bool {
    //~^ ERROR bool: rocket::response::Responder
    false
}

#[catch(404)]
//~^ ERROR mismatched types
fn f3(_request: bool) -> usize {
    //~^ ERROR usize: rocket::response::Responder
    10
}

#[catch(404)]
fn f4() -> usize {
    //~^ ERROR usize: rocket::response::Responder
    10
}

fn main() {  }
