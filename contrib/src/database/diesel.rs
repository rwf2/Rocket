extern crate diesel;
extern crate r2d2_diesel;

use std::ops::Deref;

use rocket::State;
use rocket::http::Status;
use rocket::request::{Request, FromRequest, Outcome};
use rocket::outcome::Outcome::*;

use self::diesel::{Connection as DieselConnection, ConnectionError};

use super::{Config, Pool, PooledConnection, InitError};
use super::state::Storage;

use self::r2d2_diesel::ConnectionManager;

#[derive(Debug)]
pub enum DieselPoolError {
    Connect(ConnectionError),
    Init(InitError),
}

impl From<ConnectionError> for DieselPoolError {
    fn from(e: ConnectionError) -> Self {
        DieselPoolError::Connect(e)
    }
}

impl From<InitError> for DieselPoolError {
    fn from(e: InitError) -> Self {
        DieselPoolError::Init(e)
    }
}

static DieselPool: Storage<Pool<ConnectionManager<T: DieselConnection>>> = Storage::new();

type DieselPooledConnection<T> = PooledConnection<ConnectionManager<T>>;

pub struct Connection<T: DieselConnection + Send + 'static>(DieselPooledConnection<T>);

impl<T> Connection<T> where
    T: DieselConnection + Send + 'static
{
    pub fn init(url: &str) -> Result<(), DieselPoolError> {
        let r2d2_config = Config::default();
        let r2d2_manager = ConnectionManager::<T>::new(url);
        DieselPool.set(Pool::new(r2d2_config, r2d2_manager)?);
        Ok(())
    }
}

impl<T> Deref for Connection<T> where
    T: DieselConnection + Send + 'static
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, 'r, T> FromRequest<'a, 'r> for Connection<T> where
    T: DieselConnection + Send + 'static
{
    type Error = ();

    fn from_request(request: &'a Request<'r>)
                    -> Outcome<Self, Self::Error> {
        let pool = match <State<DieselPool<T>> as FromRequest>::from_request(request) {
            Success(pool) => pool,
            Failure(e) => return Failure(e),
            Forward(_) => return Forward(()),
        };

        match pool.get() {
            Ok(conn) => Success(Connection(conn)),
            Err(e) => {
                error_!("Postgres connection error: {:?}", e);
                Failure((Status::InternalServerError, ()))
            }
        }
    }
}

