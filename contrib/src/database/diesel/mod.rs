#[cfg(feature = "diesel_pg")]
mod postgres;

#[cfg(feature = "diesel_pg")]
pub use self::postgres::PostgresDatabase;
