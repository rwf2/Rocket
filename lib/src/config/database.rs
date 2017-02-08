use super::ConfigError;
use super::Environment::*;

use std::fmt;
use std::str::FromStr;
use std::env;

use self::DatabaseType::*;

pub struct Database {
    pub name: String,
    pub database_type: DatabaseType,
}

/// An enum corresponding to the valid database configuration.
#[derive(Hash, PartialEq, Eq, Debug, Clone, Copy)]
pub enum DatabaseType {
    /// The PostgreSQL database connection.
    Postgres,
    /// The MySQL database connection.
    Mysql,
    /// The Diesel database connection.
    Diesel,
}

impl DatabaseType {
    /// Returns a string with a comma-separated list of valid databases.
    pub(crate) fn valid() -> &'static str {
        "postgres, mysql, diesel"
    }
}

impl FromStr for DatabaseType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let db = match s {
            "postgres" | "postgresql" | "pg" => Postgres,
            "mysql" => Mysql,
            "diesel" => Diesel,
            _ => return Err(()),
        };

        Ok(db)
    }
}

impl fmt::Display for DatabaseType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Postgres => write!(f, "postgres"),
            Mysql => write!(f, "mysql"),
            Diesel => write!(f, "diesel"),
        }
    }
}
