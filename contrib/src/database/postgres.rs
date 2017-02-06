extern crate postgres;
extern crate r2d2_postgres;

use rocket::outcome::Outcome;
use rocket::request::{Request, FromRequest};
use rocket::state::Storage;

pub struct PostgresDatabase;

impl PostgresDatabase {

}

impl<'a, 'r> FromRequest<'a, 'r> for PostgresDatabase {
    type Error = GetTimeout;
    fn from_request(_: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        
    }
}
