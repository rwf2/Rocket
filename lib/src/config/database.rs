use super::ConfigError;
use super::Environment::*;

use std::fmt;
use std::str::FromStr;
use std::env;
use std::time::Duration;

use self::ConnectionType::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectionConfig {
    /// The unique conenction name.
    pub name: String,
    /// The url to access database.
    pub url: String,
    /// The connection type.
    pub connection_type: ConnectionType,
}

/// An enum corresponding to the valid database configuration.
#[derive(Hash, PartialEq, Eq, Debug, Clone, Copy)]
pub enum ConnectionType {
    /// The PostgreSQL database connection.
    Postgres,
    /// The MySQL database connection.
    Mysql,
    /// The Diesel database connection.
    Diesel,
}

impl ConnectionType {
    /// Returns a string with a comma-separated list of valid database connections.
    pub(crate) fn valid() -> &'static str {
        "postgres, mysql, diesel"
    }

    /// Returns a list of all of the possible database connections.
    #[inline]
    pub(crate) fn all() -> [ConnectionType; 3] {
        [Postgres, Mysql, Diesel]
    }
}

impl FromStr for ConnectionType {
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

impl fmt::Display for ConnectionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Postgres => write!(f, "postgres"),
            Mysql => write!(f, "mysql"),
            Diesel => write!(f, "diesel"),
        }
    }
}
