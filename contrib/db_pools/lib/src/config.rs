use rocket::serde::{Deserialize, Serialize};

/// Base configuration for all database drivers.
///
/// A dictionary matching this structure is extracted from the active
/// [`Figment`](crate::figment::Figment), scoped to `databases.name`, where
/// `name` is the name of the database, by the
/// [`Initializer`](crate::Initializer) fairing on ignition and used to
/// configure the relevant database and database pool.
///
/// With the default provider, these parameters are typically configured in a
/// `Rocket.toml` file:
///
/// ```toml
/// [default.databases.db_name]
/// url = "/path/to/db.sqlite"
///
/// # Only `url` is required. These have sane defaults and are optional.
/// min_connections = 64
/// max_connections = 1024
/// connect_timeout = 5
/// idle_timeout = 120
///
/// # This option is only supported by the `sqlx_sqlite` driver.
/// extensions = ["memvfs", "rot13"]
/// ```
///
/// Alternatively, a custom provider can be used. For example, a custom `Figment`
/// with a global `databases.name` configuration:
///
/// ```rust
/// # extern crate rocket_db_pools_community as rocket_db_pools;
/// # use rocket::launch;
/// #[launch]
/// fn rocket() -> _ {
///     let figment = rocket::Config::figment()
///         .merge(("databases.name", rocket_db_pools::Config {
///             url: "db:specific@config&url".into(),
///             min_connections: None,
///             max_connections: 1024,
///             connect_timeout: 3,
///             idle_timeout: None,
///             extensions: None,
///         }));
///
///     rocket::custom(figment)
/// }
/// ```
///
/// For general information on configuration in Rocket, see [`rocket::config`].
/// For higher-level details on configuring a database, see the [crate-level
/// docs](crate#configuration).
// NOTE: Defaults provided by the figment created in the `Initializer` fairing.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(crate = "rocket::serde")]
pub struct Config {
    /// Database-specific connection and configuration URL.
    ///
    /// The format of the URL is database specific; consult your database's
    /// documentation.
    pub url: String,
    /// Minimum number of connections to maintain in the pool.
    ///
    /// **Note:** `deadpool` drivers do not support and thus ignore this value.
    ///
    /// _Default:_ `None`.
    pub min_connections: Option<u32>,
    /// Maximum number of connections to maintain in the pool.
    ///
    /// _Default:_ `workers * 4`.
    pub max_connections: usize,
    /// Number of seconds to wait for a connection before timing out.
    ///
    /// If the timeout elapses before a connection can be made or retrieved from
    /// a pool, an error is returned.
    ///
    /// _Default:_ `5`.
    pub connect_timeout: u64,
    /// Maximum number of seconds to keep a connection alive for.
    ///
    /// After a connection is established, it is maintained in a pool for
    /// efficient connection retrieval. When an `idle_timeout` is set, that
    /// connection will be closed after the timeout elapses. If an
    /// `idle_timeout` is not specified, the behavior is driver specific but
    /// typically defaults to keeping a connection active indefinitely.
    ///
    /// _Default:_ `None`.
    pub idle_timeout: Option<u64>,
    /// A list of database extensions to load at run-time.
    ///
    /// **Note:** Only the `sqlx_sqlite` driver supports this option (for SQLite
    /// extensions) at this time. All other drivers ignore this option.
    ///
    /// _Default:_ `None`.
    pub extensions: Option<Vec<String>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            url: Default::default(),
            min_connections: Default::default(),
            max_connections: rocket::Config::default().workers * 4,
            connect_timeout: 5,
            idle_timeout: Default::default(),
            extensions: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn default_values_sane() {
        let config = Config::default();
        assert_ne!(config.max_connections, 0);
        assert_eq!(config.connect_timeout, 5);
    }
}
