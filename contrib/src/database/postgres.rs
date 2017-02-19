extern crate postgres as postgres_ext;
extern crate r2d2_postgres;

use std::ops::Deref;

pub use self::postgres_ext::Connection as PgConnection;
use self::postgres_ext::error::ConnectError;

use rocket::State;
use rocket::http::Status;
use rocket::request::{Request, FromRequest, Outcome};
use rocket::outcome::Outcome::*;

use self::r2d2_postgres::{PostgresConnectionManager as PgManager, TlsMode};

use super::{Config, Pool, InitError, PooledConnection};
use super::state::Storage;

#[derive(Debug)]
pub enum PostgresPoolError {
    Connect(ConnectError),
    Init(InitError),
}

impl From<ConnectError> for PostgresPoolError {
    fn from(e: ConnectError) -> Self {
        PostgresPoolError::Connect(e)
    }
}

impl From<InitError> for PostgresPoolError {
    fn from(e: InitError) -> Self {
        PostgresPoolError::Init(e)
    }
}

static PG_POOL: Storage<Pool<PgManager>> = Storage::new();

type PgPooledConnection = PooledConnection<PgManager>;

pub struct Connection(PgPooledConnection);

impl Connection {
    pub fn init(url: &str) -> Result<(), PostgresPoolError> {
        let r2d2_config = Config::default();
        let r2d2_manager = PgManager::new(url, TlsMode::None)?;
        PG_POOL.set(Pool::new(r2d2_config, r2d2_manager)?);
        Ok(())
    }
}

impl Deref for Connection {
    type Target = PgConnection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for Connection {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        let pool = PG_POOL.get();
        match pool.get() {
            Ok(conn) => Success(Connection(conn)),
            Err(e) => {
                error_!("Postgres connection error: {:?}", e);
                Failure((Status::InternalServerError, ()))
            }
        }
    }
}
