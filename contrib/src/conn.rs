use std::ops::Deref;

use r2d2::{self, ManageConnection, Pool, PooledConnection};

use rocket::http::Status;
use rocket::request::{self, FromRequest};
use rocket::{Outcome, Request, State};

pub fn init_pool<T: ManageConnection>(manager: T, max_size: u32) -> Result<Pool<T>, r2d2::Error> {
    let pool = Pool::builder()
        .max_size(max_size)
        .build(manager)?;
    Ok(pool)
}

pub struct Conn<T>
where
    T: ManageConnection + 'static,
{
    pooled_connection: PooledConnection<T>,
}

impl<T> Deref for Conn<T>
where
    T: ManageConnection + 'static,
{
    type Target = PooledConnection<T>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.pooled_connection
    }
}

impl<'a, 'r, T> FromRequest<'a, 'r> for Conn<T>
where
    T: ManageConnection + 'static,
{
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Conn<T>, ()> {
        let pool = request.guard::<State<Pool<T>>>()?;

        match pool.get() {
            Ok(conn) => Outcome::Success(Conn { pooled_connection: conn }),
            Err(_) => Outcome::Failure((Status::ServiceUnavailable, ()))
        }
    }
}
