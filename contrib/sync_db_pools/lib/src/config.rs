use rocket::{Rocket, Build};
use rocket::figment::{self, Figment, providers::Serialized};

use serde::{Serialize, Deserialize};

/// A base `Config` for any `Poolable` type.
///
/// For the following configuration:
///
/// ```toml
/// [global.databases.my_database]
/// url = "postgres://root:root@localhost/my_database"
/// pool_size = 10
/// timeout = 5
/// ```
///
/// ...`Config::from("my_database", rocket)` would return the following struct:
///
/// ```rust
/// # use rocket_sync_db_pools::Config;
/// Config {
///     url: "postgres://root:root@localhost/my_database".into(),
///     pool_size: 10,
///     timeout: 5
/// };
/// ```
///
/// If you want to implement your own custom database adapter (or other
/// database-like struct that can be pooled by `r2d2`) and need some more
/// configurations options, you may need to define a custom `Config` struct.
/// Note, however, that the configuration values in `Config` are required.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Config {
    /// Connection URL specified in the Rocket configuration.
    pub url: String,
    /// Initial pool size. Defaults to the number of Rocket workers * 4.
    pub pool_size: u32,
    /// How long to wait, in seconds, for a new connection before timing out.
    /// Defaults to `5`.
    // FIXME: Use `time`.
    pub timeout: u8,
}

impl Config {
    /// Retrieves the database configuration for the database named `name`.
    ///
    /// This function is primarily used by the generated code from the
    /// `#[database]` attribute.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[cfg(feature = "diesel_sqlite_pool")] {
    /// # use rocket::figment::{Figment, providers::{Format, Toml}};
    /// // Assume that these are the contents of `Rocket.toml`:
    /// # let toml = Toml::string(r#"
    /// [default.databases]
    /// my_db = { url = "db/db.sqlite", pool_size = 25 }
    /// my_other_db = { url = "mysql://root:root@localhost/database" }
    /// # "#).nested();
    ///
    /// use rocket::{Rocket, Build};
    /// use rocket_sync_db_pools::Config;
    ///
    /// fn pool(rocket: &Rocket<Build>) {
    ///     let config = Config::from("my_db", rocket).unwrap();
    ///     assert_eq!(config.url, "db/db.sqlite");
    ///     assert_eq!(config.pool_size, 25);
    ///
    ///     let config = Config::from("my_other_db", rocket).unwrap();
    ///     assert_eq!(config.url, "mysql://root:root@localhost/database");
    ///
    ///     let workers = rocket.figment().extract_inner::<u32>(rocket::Config::WORKERS);
    ///     assert_eq!(config.pool_size, (workers.unwrap() * 4));
    ///
    ///     let config = Config::from("unknown_db", rocket);
    ///     assert!(config.is_err())
    /// }
    /// #
    /// # let config = Figment::from(rocket::Config::default()).merge(toml);
    /// # let rocket = rocket::custom(config);
    /// # pool(&rocket);
    /// # }
    /// ```
    pub fn from(db_name: &str, rocket: &Rocket<Build>) -> Result<Config, figment::Error> {
        Config::figment(db_name, rocket).extract::<Self>()
    }

    /// Returns a `Figment` focused on the configuration for the database with
    /// name `db_name`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::{Rocket, Build};
    /// use rocket_sync_db_pools::Config;
    ///
    /// fn pool(rocket: &Rocket<Build>) {
    ///     let my_db_figment = Config::figment("my_db", rocket);
    ///     let mysql_prod_figment = Config::figment("mysql_prod", rocket);
    /// }
    /// ```
    pub fn figment(db_name: &str, rocket: &Rocket<Build>) -> Figment {
        let db_key = format!("databases.{}", db_name);
        let default_pool_size = rocket.figment()
            .extract_inner::<u32>(rocket::Config::WORKERS)
            .map(|workers| workers * 4)
            .ok();

        let figment = Figment::from(rocket.figment())
            .focus(&db_key)
            .join(Serialized::default("timeout", 5));

        match default_pool_size {
            Some(pool_size) => figment.join(Serialized::default("pool_size", pool_size)),
            None => figment
        }
    }
}
