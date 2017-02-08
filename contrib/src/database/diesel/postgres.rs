extern crate diesel;

use self::diesel::pg::PgConnection;

pub struct Connection(PgConnection);
