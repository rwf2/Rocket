use rocket::serde::{Deserialize, Serialize};

use crate::Error;

/// A base `Config` for any `Pool` type.
///
/// For the following configuration:
///
/// ```toml
/// [global.databases.my_database]
/// url = "postgres://root:root@localhost/my_database"
/// pool_size = 10
/// ```
///
/// ...the following struct would be passed to [`Pool::initialize()`]:
///
/// ```rust
/// # use rocket_db_pools::Config;
/// Config {
///     url: "postgres://root:root@localhost/my_database".into(),
///     pool_size: Some(10),
/// };
/// ```
///
/// If you want to implement your own custom database adapter and need some more
/// configuration options, you may need to define a custom `Config` struct.
///
/// [`Pool::initialize()`]: crate::Pool::initialize
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(crate = "rocket::serde")]
pub struct Config {
    /// Connection URL specified in the Rocket configuration.
    pub url: String,
    /// Requested pool size. Defaults to the number of Rocket workers * 4.
    pub pool_size: Option<i64>,
    // TODO: timeout?
}

impl Config {
    /// Returns the requested pool size, or `default`
    pub fn pool_size_or_default<E>(&self, default: i64) -> Result<usize, Error<E>> {
        use std::convert::TryInto;
        match self.pool_size.unwrap_or(default).try_into() {
            Ok(p) => Ok(p),
            Err(_) => Err(Error::config("pool_size was outside the valid range")),
        }
    }
}
