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
/// # only `url` is required. `Initializer` provides defaults for the rest.
/// min_connections = 64
/// max_connections = 1024
/// connect_timeout = 5
/// idle_timeout = 120
/// ```
///
/// Alternatively, a custom provider can be used. For example, a custom `Figment`
/// with a global `databases.name` configuration:
///
/// ```rust
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
///         }));
///
///     rocket::custom(figment)
/// }
/// ```
///
/// For general information on configuration in Rocket, see [`rocket::config`].
/// For higher-level details on configuring a database, see the [crate-level
/// docs](crate#configuration).
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
}
