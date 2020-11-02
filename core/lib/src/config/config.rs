use std::net::{IpAddr, Ipv4Addr};

use figment::{Figment, Profile, Provider, Metadata, error::Result};
use figment::providers::{Serialized, Env, Toml, Format};
use figment::value::{Map, Dict};
use serde::{Deserialize, Serialize};
use yansi::Paint;

use crate::config::{SecretKey, TlsConfig, LogLevel};
use crate::data::Limits;

/// Rocket server configuration.
///
/// See the [module level docs](crate::config) as well as the [configuration
/// guide] for further details.
///
/// [configuration guide]: https://rocket.rs/master/guide/configuration/
///
/// # Defaults
///
/// All configuration values have a default, documented in the [fields](#fields)
/// section below. [`Config::debug_default()`] returns the default values for
/// the debug profile while [`Config::release_default()`] the default values for
/// the release profile. The [`Config::default()`] method automatically selects
/// the appropriate of the two based on the selected profile. With the exception
/// of `log_level`, which is `normal` in `debug` and `critical` in `release`,
/// and `secret_key`, which is regenerated from a random value if not set in
/// "debug" mode only, all of the values are identical in either profile.
///
/// # Provider Details
///
/// `Config` is a Figment [`Provider`] with the following characteristics:
///
///   * **Profile**
///
///     The selected profile is the value of the `ROCKET_PROFILE` environment
///     variable. If the environment variable is not set, the profile is
///     selected based on whether compilation is in debug mode, where "debug" is
///     selected, or release mode, where "release" is selected.
///     [`Config::DEBUG_PROFILE`] and [`Config::RELEASE_PROFILE`] encode these
///     values as constants, while [`Config::DEFAULT_PROFILE`] selects the
///     appropriate of the two at compile-time.
///
///   * **Metadata**
///
///     This provider is named `Rocket Config`. It does not specify a
///     [`Source`](figment::Source) and uses default interpolatation.
///
///   * **Data**
///
///     The data emitted by this provider are the keys and values corresponding
///     to the fields and values of the structure. The dictionary is emitted to
///     the "default" meta-profile.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Config {
    /// IP address to serve on. **(default: `127.0.0.1`)**
    pub address: IpAddr,
    /// Port to serve on. **(default: `8000`)**
    pub port: u16,
    /// Number of threads to use for executing futures. **(default: `cores * 2`)**
    pub workers: u16,
    /// Keep-alive timeout in seconds; disabled when `0`. **(default: `5`)**
    pub keep_alive: u32,
    /// Max level to log. **(default: _debug_ `normal` / _release_ `critical`)**
    pub log_level: LogLevel,
    /// Whether to use colors and emoji when logging. **(default: `true`)**
    #[serde(deserialize_with = "figment::util::bool_from_str_or_int")]
    pub cli_colors: bool,
    /// The secret key for signing and encrypting. **(default: `0`)**
    pub secret_key: SecretKey,
    /// The TLS configuration, if any. **(default: `None`)**
    pub tls: Option<TlsConfig>,
    /// Streaming read size limits. **(default: [`Limits::default()`])**
    pub limits: Limits,
    /// Whether `ctrl-c` initiates a server shutdown. **(default: `true`)**
    #[serde(deserialize_with = "figment::util::bool_from_str_or_int")]
    pub ctrlc: bool,
}

impl Default for Config {
    /// Returns the default configuration based on the compilation profile. This
    /// is [`Config::debug_default()`] in `debug` and
    /// [`Config::release_default()`] in `release`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Config;
    ///
    /// let config = Config::default();
    /// ```
    fn default() -> Config {
        #[cfg(debug_assertions)] { Config::debug_default() }
        #[cfg(not(debug_assertions))] { Config::release_default() }
    }
}

impl Config {
    /// The default "debug" profile.
    pub const DEBUG_PROFILE: Profile = Profile::const_new("debug");

    /// The default "release" profile.
    pub const RELEASE_PROFILE: Profile = Profile::const_new("release");

    /// The default profile: "debug" on `debug`, "release" on `release`.
    #[cfg(debug_assertions)]
    pub const DEFAULT_PROFILE: Profile = Self::DEBUG_PROFILE;

    /// The default profile: "debug" on `debug`, "release" on `release`.
    #[cfg(not(debug_assertions))]
    pub const DEFAULT_PROFILE: Profile = Self::RELEASE_PROFILE;

    const DEPRECATED_KEYS: &'static [(&'static str, Option<&'static str>)] = &[
        ("env", Some("profile")), ("log", Some("log_level")),
    ];

    /// Returns the default configuration for the `debug` profile, irrespective
    /// of the compilation profile. For the default Rocket will use, which is
    /// chosen based on the configuration profile, call [`Config::default()`].
    /// See [Defaults](#Defaults) for specifics.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Config;
    ///
    /// let config = Config::debug_default();
    /// ```
    pub fn debug_default() -> Config {
        Config {
            address: Ipv4Addr::new(127, 0, 0, 1).into(),
            port: 8000,
            workers: num_cpus::get() as u16 * 2,
            keep_alive: 5,
            log_level: LogLevel::Normal,
            cli_colors: true,
            secret_key: SecretKey::zero(),
            tls: None,
            limits: Limits::default(),
            ctrlc: true,
        }
    }

    /// Returns the default configuration for the `release` profile,
    /// irrespective of the compilation profile. For the default Rocket will
    /// use, which is chosen based on the configuration profile, call
    /// [`Config::default()`]. See [Defaults](#Defaults) for specifics.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Config;
    ///
    /// let config = Config::release_default();
    /// ```
    pub fn release_default() -> Config {
        Config {
            log_level: LogLevel::Critical,
            ..Config::debug_default()
        }
    }

    /// Returns the default provider figment used by [`rocket::ignite()`].
    ///
    /// The default figment reads from the following sources, in ascending
    /// priority order:
    ///
    ///   1. [`Config::default()`] (see [Defaults](#Defaults))
    ///   2. `Rocket.toml` _or_ filename in `ROCKET_CONFIG` environment variable
    ///   3. `ROCKET_` prefixed environment variables
    ///
    /// The profile selected is the value set in the `ROCKET_PROFILE`
    /// environment variable. If it is not set, it defaults to `debug` when
    /// compiled in debug mode and `release` when compiled in release mode.
    ///
    /// [`rocket::ignite()`]: crate::ignite()
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Config;
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize)]
    /// struct MyConfig {
    ///     app_key: String,
    /// }
    ///
    /// let my_config = Config::figment().extract::<MyConfig>();
    /// ```
    pub fn figment() -> Figment {
        Figment::from(Config::default())
            .merge(Toml::file(Env::var_or("ROCKET_CONFIG", "Rocket.toml")).nested())
            .merge(Env::prefixed("ROCKET_").ignore(&["PROFILE"]).global())
    }

    /// Attempts to extract a `Config` from `provider`.
    ///
    /// # Panics
    ///
    /// If extraction fails, prints an error message indicating the failure and
    /// panics.
    ///
    /// # Example
    ///
    /// ```rust
    /// use figment::{Figment, providers::{Toml, Format, Env}};
    ///
    /// // Use Rocket's default `Figment`, but allow values from `MyApp.toml`
    /// // and `MY_APP_` prefixed environment variables to supersede its values.
    /// let figment = rocket::Config::figment()
    ///     .merge(Toml::file("MyApp.toml").nested())
    ///     .merge(Env::prefixed("MY_APP_"));
    ///
    /// let config = rocket::Config::from(figment);
    /// ```
    pub fn from<T: Provider>(provider: T) -> Self {
        // Check for now depreacted config values.
        let figment = Figment::from(&provider);
        for (key, replacement) in Self::DEPRECATED_KEYS {
            if figment.find_value(key).is_ok() {
                let span = warn_span!("deprecated_key", "found value for deprecated config key {}", Paint::white(key));
                if let Some(new_key) = replacement {
                    info!(parent: &span, "key has been by replaced by {}", Paint::white(new_key));
                }
            }
        }

        #[allow(unused_mut)]
        let mut config = figment.extract::<Self>().unwrap_or_else(|e| {
            pretty_print_error(e);
            panic!("aborting due to configuration error(s)")
        });

        #[cfg(all(feature = "secrets", not(test), not(rocket_unsafe_secret_key)))] {
            if config.secret_key.is_zero() {
                if figment.profile() != Self::DEBUG_PROFILE {
                    crate::trace::try_init(LogLevel::Debug, true);
                    let span = error_span!("secrets enabled in `release` without `secret_key`");
                    info!(parent: &span, "disable `secrets` feature or configure a `secret_key`");
                    panic!("aborting due to configuration error(s)")
                } else {
                    let span = warn_span!("secrets enabled in `debug` without `secret_key`");
                    info!(parent: &span, "disable `secrets` feature or configure a `secret_key`");
                    info!(parent: &span, "this becomes a hard error in `release`");
                }

                // in debug, generate a key for a bit more security
                config.secret_key = SecretKey::generate().unwrap_or(SecretKey::zero());
            }
        }

        config
    }

    /// Returns `true` if TLS is enabled.
    ///
    /// TLS is enabled when the `tls` feature is enabled and TLS has been
    /// configured.
    ///
    /// # Example
    ///
    /// ```rust
    /// let config = rocket::Config::default();
    /// if config.tls_enabled() {
    ///     println!("TLS is enabled!");
    /// } else {
    ///     println!("TLS is disabled.");
    /// }
    /// ```
    pub fn tls_enabled(&self) -> bool {
        cfg!(feature = "tls") && self.tls.is_some()
    }

    pub(crate) fn pretty_print(&self, profile: &Profile) {
        use crate::trace::PaintExt;

        let span = info_span!("configured", "{}Configured for {}", Paint::emoji("🔧 "), profile);
        let _e = span.enter();

        info!(address = %&self.address);
        info!(port = %&self.port);
        info!(workers = %self.workers);
        info!(log_level = %self.log_level);
        info!(secret_key = ?&self.secret_key);
        info!(limits = %&self.limits);
        info!(cli_colors = %&self.cli_colors);

        let ka = self.keep_alive;
        if ka > 0 {
            info!(keep_alive = %Paint::default(format!("{}s", ka)).bold());
        } else {
            info!(keep_alive = %Paint::default("disabled").bold());
        }

        match self.tls_enabled() {
            true => info!(tls = %Paint::default("enabled").bold()),
            false => info!(tls = %Paint::default("disabled").bold()),
        }
    }
}

impl Provider for Config {
    fn metadata(&self) -> Metadata {
        Metadata::named("Rocket Config")
    }

    #[track_caller]
    fn data(&self) -> Result<Map<Profile, Dict>> {
        Serialized::defaults(self).data()
    }

    fn profile(&self) -> Option<Profile> {
        Some(Profile::from_env_or("ROCKET_PROFILE", Self::DEFAULT_PROFILE))
    }
}

#[doc(hidden)]
pub fn pretty_print_error(error: figment::Error) {
    crate::trace::try_init(LogLevel::Debug, true);

    for e in error {
        let span = error_span!("config_error", "{}", e.kind);

        if let (Some(ref profile), Some(ref md)) = (&e.profile, &e.metadata) {
            if !e.path.is_empty() {
                let key = md.interpolate(profile, &e.path);
                info!(parent: &span, key = %Paint::white(&key));
            }
        }

        if let Some(ref md) = e.metadata {
            if let Some(ref source) = md.source {
                info!(parent: &span, "in {} {}", Paint::white(source), md.name);
            }
        }
    }
}
