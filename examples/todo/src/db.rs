use std::ops::Deref;

use diesel::sqlite::SqliteConnection;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};

use rocket::http::Status;
use rocket::request::{self, FromRequest};
use rocket::{Request, State, Outcome};

pub type SqlitePool = Pool<ConnectionManager<SqliteConnection>>;

pub const DATABASE_URL: &'static str = env!("DATABASE_URL");

pub fn init_pool() -> SqlitePool {
    let manager = ConnectionManager::<SqliteConnection>::new(DATABASE_URL);
    Pool::new(manager).expect("db pool")
}

pub struct Conn(pub PooledConnection<ConnectionManager<SqliteConnection>>);

impl Deref for Conn {
    type Target = SqliteConnection;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for Conn {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Conn, ()> {
        let pool = request.guard::<State<SqlitePool>>()?;
        match pool.get() {
            Ok(conn) => Outcome::Success(Conn(conn)),
            Err(_) => Outcome::Failure((Status::ServiceUnavailable, ()))
        }
    }
}
