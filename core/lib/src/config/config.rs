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
/// "debug" mode only, all default values are identical in all profiles.
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
    /// Number of future-executing threads. **(default: `num cores`)**
    pub workers: usize,
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

    const DEPRECATED_PROFILES: &'static [(&'static str, Option<&'static str>)] = &[
        ("dev", Some("debug")), ("prod", Some("release")),
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
            workers: num_cpus::get(),
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
        let figment = Figment::from(&provider);

        #[allow(unused_mut)]
        let mut config = figment.extract::<Self>().unwrap_or_else(|e| {
            pretty_print_error(e);
            panic!("aborting due to configuration error(s)")
        });

        #[cfg(all(feature = "secrets", not(test), not(rocket_unsafe_secret_key)))]
        if !config.secret_key.is_provided() {
            if figment.profile() != Self::DEBUG_PROFILE {
                crate::logger::try_init(LogLevel::Debug, true, false);
                error!("secrets enabled in non-`debug` without `secret_key`");
                info_!("disable `secrets` feature or configure a `secret_key`");
                panic!("aborting due to configuration error(s)")
            }

            // in debug, generate a key for a bit more security
            config.secret_key = SecretKey::generate().unwrap_or(SecretKey::zero());
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

    pub(crate) fn pretty_print(&self, figment: &Figment) {
        use crate::logger::PaintExt;

        launch_info!("{}Configured for {}.", Paint::emoji("🔧 "), figment.profile());

        launch_info_!("address: {}", Paint::default(&self.address).bold());
        launch_info_!("port: {}", Paint::default(&self.port).bold());
        launch_info_!("workers: {}", Paint::default(self.workers).bold());
        launch_info_!("log level: {}", Paint::default(self.log_level).bold());
        launch_info_!("secret key: {:?}", Paint::default(&self.secret_key).bold());
        launch_info_!("limits: {}", Paint::default(&self.limits).bold());
        launch_info_!("cli colors: {}", Paint::default(&self.cli_colors).bold());

        let ka = self.keep_alive;
        if ka > 0 {
            launch_info_!("keep-alive: {}", Paint::default(format!("{}s", ka)).bold());
        } else {
            launch_info_!("keep-alive: {}", Paint::default("disabled").bold());
        }

        match self.tls_enabled() {
            true => launch_info_!("tls: {}", Paint::default("enabled").bold()),
            false => launch_info_!("tls: {}", Paint::default("disabled").bold()),
        }

        #[cfg(all(feature = "secrets", not(test), not(rocket_unsafe_secret_key)))]
        if !self.secret_key.is_provided() {
            warn!("secrets enabled without a configured `secret_key`");
            info_!("disable `secrets` feature or configure a `secret_key`");
            info_!("this becomes a {} in non-debug profiles", Paint::red("hard error").bold());
        }

        // Check for now depreacted config values.
        for (key, replacement) in Self::DEPRECATED_KEYS {
            if let Some(md) = figment.find_metadata(key) {
                warn!("found value for deprecated config key `{}`", Paint::white(key));
                if let Some(ref source) = md.source {
                    info_!("in {} {}", Paint::white(source), md.name);
                }

                if let Some(new_key) = replacement {
                    info_!("key has been by replaced by `{}`", Paint::white(new_key));
                }
            }
        }

        // Check for now removed config values.
        for (prefix, replacement) in Self::DEPRECATED_PROFILES {
            if let Some(profile) = figment.profiles().find(|p| p.starts_with(prefix)) {
                warn!("found set deprecated profile `{}`", Paint::white(profile));

                if let Some(new_profile) = replacement {
                    info_!("profile has been by replaced by `{}`", Paint::white(new_profile));
                } else {
                    info_!("profile `{}` has no special meaning", profile);
                }
            }
        }
    }
}

impl Provider for Config {
    fn metadata(&self) -> Metadata {
        Metadata::named("Rocket Config")
    }

    #[track_caller]
    fn data(&self) -> Result<Map<Profile, Dict>> {
        let mut map: Map<Profile, Dict> = Serialized::defaults(self).data()?;
        // We need to special-case `secret_key` since its serializer zeroes.
        if !self.secret_key.is_zero() {
            if let Some(map) = map.get_mut(&Profile::Default) {
                map.insert("secret_key".into(), self.secret_key.master().into());
            }
        }

        Ok(map)
    }

    fn profile(&self) -> Option<Profile> {
        Some(Profile::from_env_or("ROCKET_PROFILE", Self::DEFAULT_PROFILE))
    }
}

#[doc(hidden)]
pub fn pretty_print_error(error: figment::Error) {
    use figment::error::{Kind, OneOf};

    crate::logger::try_init(LogLevel::Debug, true, false);

    for e in error {
        fn w<T: std::fmt::Display>(v: T) -> Paint<T> { Paint::white(v) }

        match e.kind {
            Kind::Message(msg) => error_!("{}", msg),
            Kind::InvalidType(v, exp) => {
                error_!("invalid type: found {}, expected {}", w(v), w(exp));
            }
            Kind::InvalidValue(v, exp) => {
                error_!("invalid value {}, expected {}", w(v), w(exp));
            },
            Kind::InvalidLength(v, exp) => {
                error_!("invalid length {}, expected {}", w(v), w(exp))
            },
            Kind::UnknownVariant(v, exp) => {
                error_!("unknown variant: found `{}`, expected `{}`", w(v), w(OneOf(exp)))
            }
            Kind::UnknownField(v, exp) => {
                error_!("unknown field: found `{}`, expected `{}`", w(v), w(OneOf(exp)))
            }
            Kind::MissingField(v) => {
                error_!("missing field `{}`", w(v))
            }
            Kind::DuplicateField(v) => {
                error_!("duplicate field `{}`", w(v))
            }
            Kind::ISizeOutOfRange(v) => {
                error_!("signed integer `{}` is out of range", w(v))
            }
            Kind::USizeOutOfRange(v) => {
                error_!("unsigned integer `{}` is out of range", w(v))
            }
            Kind::Unsupported(v) => {
                error_!("unsupported type `{}`", w(v))
            }
            Kind::UnsupportedKey(a, e) => {
                error_!("unsupported type `{}` for key: must be `{}`", w(a), w(e))
            }
        }

        if let (Some(ref profile), Some(ref md)) = (&e.profile, &e.metadata) {
            if !e.path.is_empty() {
                let key = md.interpolate(profile, &e.path);
                info_!("for key {}", w(key));
            }
        }

        if let Some(md) = e.metadata {
            if let Some(source) = md.source {
                info_!("in {} {}", w(source), md.name);
            } else {
                info_!("in {}", w(md.name));
            }
        }
    }
}
