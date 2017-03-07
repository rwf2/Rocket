extern crate r2d2;
extern crate state;

use std::ops::Deref;

use rocket::State;
use rocket::http::Status;
use rocket::request::{Request, FromRequest, Outcome};
use rocket::outcome::Outcome::*;

use self::state::Storage;

#[macro_export]
macro_rules! state_pool {
    ($name: ident, $manager: expr, $conn: ty) => {
        static POOL: Storage<Pool<$manager>>
        pub struct $name(PooledConnection<$manager>);

        impl $name {
            pub fn init(url: &str) -> Result<(),> {
                let r2d2_config = Config::default();
                let r2d2_manager = $manager::new(url)?;
                PG_POOL.set(Pool::new(r2d2_config, r2d2_manager)?);
                Ok(())
            }
        }

        impl Deref for $name {
            type Target = $conn;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl<'a, 'r> FromRequest<'a, 'r> for $name {
            type Error = ();

            fn from_request(req: &'a Request<'r>) -> Outcome<Self, Self::Error> {
                

                match pool.0.get() {
                    Ok(conn) => Success($name(conn)),
                    Err(_) => Failure((Status::InternalServerError, ())),
                }
            }
        }
    }
}

#[cfg(feature = "diesel_db")]
pub mod diesel;
#[cfg(feature = "postgres_db")]
pub mod postgres;
#[cfg(feature = "redis_db")]
pub mod redis;
#[cfg(feature = "mysql_db")]
pub mod mysql;

use self::r2d2::{Config, Pool, PooledConnection, InitializationError as InitError};

