#[macro_use] extern crate rocket;

use rocket::Request;

#[catch(404)]
struct Catcher(String);
//~^ ERROR expected `fn`
//~^^ HELP on functions

#[catch(404)]
const CATCH: &str = "Catcher";
//~^ ERROR expected `fn`
//~^^ HELP on functions

#[catch("404")] //~ ERROR expected unsigned integer literal
//~^ HELP #[catch(404)]
fn e1(_request: &Request) { }

#[catch(code = "404")] //~ ERROR unexpected keyed parameter
//~^ HELP #[catch(404)]
fn e2(_request: &Request) { }

#[catch(code = 404)] //~ ERROR unexpected keyed parameter
//~^ HELP #[catch(404)]
fn e3(_request: &Request) { }

#[catch(99)] //~ ERROR in range [100, 599]
//~^ HELP #[catch(404)]
fn e4(_request: &Request) { }

#[catch(600)] //~ ERROR in range [100, 599]
//~^ HELP #[catch(404)]
fn e5(_request: &Request) { }

#[catch(400, message = "foo")] //~ ERROR unexpected attribute parameter: `message`
//~^ HELP #[catch(404)]
fn e5(_request: &Request) { }

#[catch(404)]
fn f3(_request: &Request, other: bool) {
    //~^ ERROR invalid number of arguments
    //~^^ HELP optionally take an argument
}

fn main() { }
