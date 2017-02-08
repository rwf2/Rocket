extern crate postgres;
extern crate r2d2_postgres;

use rocket::request::{Request, FromRequest, Outcome};
use rocket::outcome::Outcome::*;

use super::r2d2::GetTimeout;

pub struct Connection;

impl<'a, 'r> FromRequest<'a, 'r> for Connection {
    type Error = GetTimeout;
    fn from_request(_: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        Success(Connection)
    }
}
