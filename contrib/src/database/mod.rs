extern crate r2d2;

#[cfg(any(feature = "diesel_pg", feature = "diesel_sqlite"))]
pub mod diesel;
#[cfg(feature = "postgres_db")]
pub mod postgres;
#[cfg(feature = "redis_db")]
pub mod redis;
#[cfg(feature = "mysql_db")]
pub mod mysql;
