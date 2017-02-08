extern crate rocket;
#[macro_use] extern crate rocket_contrib;

use rocket_contrib::postgres::Connection;

fn main() {
    rocket::ignite()
        .launch();
}
