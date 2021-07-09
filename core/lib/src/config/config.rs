use std::net::{IpAddr, Ipv4Addr};

use figment::{Figment, Profile, Provider, Metadata, error::Result};
use figment::providers::{Serialized, Env, Toml, Format};
use figment::value::{Map, Dict, magic::RelativePathBuf};
use serde::{Deserialize, Serialize};
use yansi::Paint;

use crate::config::{LogLevel, Shutdown, Ident};
use crate::request::{self, Request, FromRequest};
use crate::data::Limits;

#[cfg(feature = "tls")]
use crate::config::TlsConfig;

#[cfg(feature = "secrets")]
use crate::config::SecretKey;

/// Rocket server configuration.
///
/// See the [module level docs](crate::config) as well as the [configuration
/// guide] for further details.
///
/// [configuration guide]: https://rocket.rs/v0.5-rc/guide/configuration/
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
///     The profile is set to the value of the `profile` field.
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
///
/// Note that these behaviors differ from those of [`Config::figment()`].
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Config {
    /// The selected profile. **(default: _debug_ `debug` / _release_ `release`)**
    ///
    /// **Note:** This field is never serialized nor deserialized. When a
    /// `Config` is merged into a `Figment` as a `Provider`, this profile is
    /// selected on the `Figment`. When a `Config` is extracted, this field is
    /// set to the extracting Figment's selected `Profile`.
    #[serde(skip)]
    pub profile: Profile,
    /// IP address to serve on. **(default: `127.0.0.1`)**
    pub address: IpAddr,
    /// Port to serve on. **(default: `8000`)**
    pub port: u16,
    /// Number of threads to use for executing futures. **(default: `num_cores`)**
    pub workers: usize,
    /// How, if at all, to identify the server via the `Server` header.
    /// **(default: `"Rocket"`)**
    pub ident: Ident,
    /// Streaming read size limits. **(default: [`Limits::default()`])**
    pub limits: Limits,
    /// Directory to store temporary files in. **(default:
    /// [`std::env::temp_dir()`])**
    #[serde(serialize_with = "RelativePathBuf::serialize_relative")]
    pub temp_dir: RelativePathBuf,
    /// Keep-alive timeout in seconds; disabled when `0`. **(default: `5`)**
    pub keep_alive: u32,
    /// The TLS configuration, if any. **(default: `None`)**
    #[cfg(feature = "tls")]
    #[cfg_attr(nightly, doc(cfg(feature = "tls")))]
    pub tls: Option<TlsConfig>,
    /// The secret key for signing and encrypting. **(default: `0`)**
    ///
    /// **Note:** This field _always_ serializes as a 256-bit array of `0`s to
    /// aid in preventing leakage of the secret key.
    #[cfg(feature = "secrets")]
    #[cfg_attr(nightly, doc(cfg(feature = "secrets")))]
    #[serde(serialize_with = "SecretKey::serialize_zero")]
    pub secret_key: SecretKey,
    /// Graceful shutdown configuration. **(default: [`Shutdown::default()`])**
    pub shutdown: Shutdown,
    /// Max level to log. **(default: _debug_ `normal` / _release_ `critical`)**
    pub log_level: LogLevel,
    /// Whether to use colors and emoji when logging. **(default: `true`)**
    #[serde(deserialize_with = "figment::util::bool_from_str_or_int")]
    pub cli_colors: bool,
    /// PRIVATE: This structure may grow (but never change otherwise) in a
    /// non-breaking release. As such, constructing this structure should
    /// _always_ be done using a public constructor or update syntax:
    ///
    /// ```rust
    /// use rocket::Config;
    ///
    /// let config = Config {
    ///     port: 1024,
    ///     keep_alive: 10,
    ///     ..Default::default()
    /// };
    /// ```
    #[doc(hidden)]
    #[serde(skip)]
    pub __non_exhaustive: (),
}

impl Default for Config {
    /// Returns the default configuration based on the Rust compilation profile.
    /// This is [`Config::debug_default()`] in `debug` and
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
    const DEPRECATED_KEYS: &'static [(&'static str, Option<&'static str>)] = &[
        ("env", Some(Self::PROFILE)), ("log", Some(Self::LOG_LEVEL)),
        ("read_timeout", None), ("write_timeout", None),
    ];

    const DEPRECATED_PROFILES: &'static [(&'static str, Option<&'static str>)] = &[
        ("dev", Some("debug")), ("prod", Some("release")), ("stag", None)
    ];

    /// Returns the default configuration for the `debug` profile, _irrespective
    /// of the Rust compilation profile_ and `ROCKET_PROFILE`.
    ///
    /// This may differ from the configuration used by default,
    /// [`Config::default()`], which is selected based on the Rust compilation
    /// profile. See [defaults](#defaults) and [provider
    /// details](#provider-details) for specifics.
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
            profile: Self::DEBUG_PROFILE,
            address: Ipv4Addr::new(127, 0, 0, 1).into(),
            port: 8000,
            workers: num_cpus::get(),
            ident: Ident::default(),
            limits: Limits::default(),
            temp_dir: std::env::temp_dir().into(),
            keep_alive: 5,
            #[cfg(feature = "tls")]
            tls: None,
            #[cfg(feature = "secrets")]
            secret_key: SecretKey::zero(),
            shutdown: Shutdown::default(),
            log_level: LogLevel::Normal,
            cli_colors: true,
            __non_exhaustive: (),
        }
    }

    /// Returns the default configuration for the `release` profile,
    /// _irrespective of the Rust compilation profile_ and `ROCKET_PROFILE`.
    ///
    /// This may differ from the configuration used by default,
    /// [`Config::default()`], which is selected based on the Rust compilation
    /// profile. See [defaults](#defaults) and [provider
    /// details](#provider-details) for specifics.
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
            profile: Self::RELEASE_PROFILE,
            log_level: LogLevel::Critical,
            ..Config::debug_default()
        }
    }

    /// Returns the default provider figment used by [`rocket::build()`].
    ///
    /// The default figment reads from the following sources, in ascending
    /// priority order:
    ///
    ///   1. [`Config::default()`] (see [defaults](#defaults))
    ///   2. `Rocket.toml` _or_ filename in `ROCKET_CONFIG` environment variable
    ///   3. `ROCKET_` prefixed environment variables
    ///
    /// The profile selected is the value set in the `ROCKET_PROFILE`
    /// environment variable. If it is not set, it defaults to `debug` when
    /// compiled in debug mode and `release` when compiled in release mode.
    ///
    /// [`rocket::build()`]: crate::build()
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
            .select(Profile::from_env_or("ROCKET_PROFILE", Self::DEFAULT_PROFILE))
    }

    /// Attempts to extract a `Config` from `provider`, returning the result.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Config;
    /// use rocket::figment::providers::{Toml, Format, Env};
    ///
    /// // Use Rocket's default `Figment`, but allow values from `MyApp.toml`
    /// // and `MY_APP_` prefixed environment variables to supersede its values.
    /// let figment = Config::figment()
    ///     .merge(("some-thing", 123))
    ///     .merge(Env::prefixed("CONFIG_"));
    ///
    /// let config = Config::try_from(figment);
    /// ```
    pub fn try_from<T: Provider>(provider: T) -> Result<Self> {
        let figment = Figment::from(provider);
        let mut config = figment.extract::<Self>()?;
        config.profile = figment.profile().clone();
        Ok(config)
    }

    /// Extract a `Config` from `provider`, panicking if extraction fails.
    ///
    /// # Panics
    ///
    /// If extraction fails, prints an error message indicating the failure and
    /// panics. For a version that doesn't panic, use [`Config::try_from()`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::Config;
    /// use rocket::figment::providers::{Toml, Format, Env};
    ///
    /// // Use Rocket's default `Figment`, but allow values from `MyApp.toml`
    /// // and `MY_APP_` prefixed environment variables to supersede its values.
    /// let figment = Config::figment()
    ///     .merge(Toml::file("MyApp.toml").nested())
    ///     .merge(Env::prefixed("MY_APP_"));
    ///
    /// let config = Config::from(figment);
    /// ```
    pub fn from<T: Provider>(provider: T) -> Self {
        Self::try_from(provider).unwrap_or_else(|e| {
            pretty_print_error(e);
            panic!("aborting due to configuration error(s)")
        })
    }

    /// Returns `true` if TLS is enabled.
    ///
    /// TLS is enabled when the `tls` feature is enabled and TLS has been
    /// configured with at least one ciphersuite. Note that without changing
    /// defaults, all supported ciphersuites are enabled in the recommended
    /// configuration.
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
        #[cfg(feature = "tls")] {
            self.tls.as_ref().map_or(false, |tls| !tls.ciphers.is_empty())
        }

        #[cfg(not(feature = "tls"))] { false }
    }

    /// Returns `true` if mTLS is enabled.
    ///
    /// mTLS is enabled when TLS is enabled ([`Config::tls_enabled()`]) _and_
    /// the `mtls` feature is enabled _and_ mTLS has been configured with a CA
    /// certificate chain.
    ///
    /// # Example
    ///
    /// ```rust
    /// let config = rocket::Config::default();
    /// if config.mtls_enabled() {
    ///     println!("mTLS is enabled!");
    /// } else {
    ///     println!("mTLS is disabled.");
    /// }
    /// ```
    pub fn mtls_enabled(&self) -> bool {
        if !self.tls_enabled() {
            return false;
        }

        #[cfg(feature = "mtls")] {
            self.tls.as_ref().map_or(false, |tls| tls.mutual.is_some())
        }

        #[cfg(not(feature = "mtls"))] { false }
    }

    pub(crate) fn pretty_print(&self, figment: &Figment) {
        use crate::log::PaintExt;

        fn bold<T: std::fmt::Display>(val: T) -> Paint<T> {
            Paint::default(val).bold()
        }

        launch_info!("{}Configured for {}.", Paint::emoji("🔧 "), self.profile);
        launch_info_!("address: {}", bold(&self.address));
        launch_info_!("port: {}", bold(&self.port));
        launch_info_!("workers: {}", bold(self.workers));
        launch_info_!("ident: {}", bold(&self.ident));
        launch_info_!("limits: {}", bold(&self.limits));
        launch_info_!("temp dir: {}", bold(&self.temp_dir.relative().display()));

        match self.keep_alive {
            0 => launch_info_!("keep-alive: {}", bold("disabled")),
            ka => launch_info_!("keep-alive: {}{}", bold(ka), bold("s")),
        }

        match (self.tls_enabled(), self.mtls_enabled()) {
            (true, true) => launch_info_!("tls: {}", bold("enabled w/mtls")),
            (true, false) => launch_info_!("tls: {} w/o mtls", bold("enabled")),
            (false, _) => launch_info_!("tls: {}", bold("disabled")),
        }

        #[cfg(feature = "secrets")] {
            launch_info_!("secret key: {}", bold(&self.secret_key));
            if !self.secret_key.is_provided() {
                warn!("secrets enabled without a stable `secret_key`");
                launch_info_!("disable `secrets` feature or configure a `secret_key`");
                launch_info_!("this becomes an {} in non-debug profiles", Paint::red("error"));
            }
        }

        launch_info_!("shutdown: {}", bold(&self.shutdown));
        launch_info_!("log level: {}", bold(self.log_level));
        launch_info_!("cli colors: {}", bold(&self.cli_colors));

        // Check for now depreacted config values.
        for (key, replacement) in Self::DEPRECATED_KEYS {
            if let Some(md) = figment.find_metadata(key) {
                warn!("found value for deprecated config key `{}`", Paint::white(key));
                if let Some(ref source) = md.source {
                    launch_info_!("in {} {}", Paint::white(source), md.name);
                }

                if let Some(new_key) = replacement {
                    launch_info_!("key has been by replaced by `{}`", Paint::white(new_key));
                } else {
                    launch_info_!("key has no special meaning");
                }
            }
        }

        // Check for now removed config values.
        for (prefix, replacement) in Self::DEPRECATED_PROFILES {
            if let Some(profile) = figment.profiles().find(|p| p.starts_with(prefix)) {
                warn!("found set deprecated profile `{}`", Paint::white(profile));

                if let Some(new_profile) = replacement {
                    launch_info_!("profile was replaced by `{}`", Paint::white(new_profile));
                } else {
                    launch_info_!("profile `{}` has no special meaning", profile);
                }
            }
        }
    }
}

/// Associated constants for default profiles.
impl Config {
    /// The default debug profile: `debug`.
    pub const DEBUG_PROFILE: Profile = Profile::const_new("debug");

    /// The default release profile: `release`.
    pub const RELEASE_PROFILE: Profile = Profile::const_new("release");

    /// The default profile: "debug" on `debug`, "release" on `release`.
    #[cfg(debug_assertions)]
    pub const DEFAULT_PROFILE: Profile = Self::DEBUG_PROFILE;

    /// The default profile: "debug" on `debug`, "release" on `release`.
    #[cfg(not(debug_assertions))]
    pub const DEFAULT_PROFILE: Profile = Self::RELEASE_PROFILE;
}

/// Associated constants for stringy versions of configuration parameters.
impl Config {
    /// The stringy parameter name for setting/extracting [`Config::profile`].
    ///
    /// This isn't `pub` because setting it directly does nothing.
    const PROFILE: &'static str = "profile";

    /// The stringy parameter name for setting/extracting [`Config::address`].
    pub const ADDRESS: &'static str = "address";

    /// The stringy parameter name for setting/extracting [`Config::port`].
    pub const PORT: &'static str = "port";

    /// The stringy parameter name for setting/extracting [`Config::workers`].
    pub const WORKERS: &'static str = "workers";

    /// The stringy parameter name for setting/extracting [`Config::keep_alive`].
    pub const KEEP_ALIVE: &'static str = "keep_alive";

    /// The stringy parameter name for setting/extracting [`Config::limits`].
    pub const LIMITS: &'static str = "limits";

    /// The stringy parameter name for setting/extracting [`Config::tls`].
    pub const TLS: &'static str = "tls";

    /// The stringy parameter name for setting/extracting [`Config::secret_key`].
    pub const SECRET_KEY: &'static str = "secret_key";

    /// The stringy parameter name for setting/extracting [`Config::temp_dir`].
    pub const TEMP_DIR: &'static str = "temp_dir";

    /// The stringy parameter name for setting/extracting [`Config::log_level`].
    pub const LOG_LEVEL: &'static str = "log_level";

    /// The stringy parameter name for setting/extracting [`Config::shutdown`].
    pub const SHUTDOWN: &'static str = "shutdown";

    /// The stringy parameter name for setting/extracting [`Config::cli_colors`].
    pub const CLI_COLORS: &'static str = "cli_colors";
}

impl Provider for Config {
    fn metadata(&self) -> Metadata {
        Metadata::named("Rocket Config")
    }

    #[track_caller]
    fn data(&self) -> Result<Map<Profile, Dict>> {
        #[allow(unused_mut)]
        let mut map: Map<Profile, Dict> = Serialized::defaults(self).data()?;

        // We need to special-case `secret_key` since its serializer zeroes.
        #[cfg(feature = "secrets")]
        if !self.secret_key.is_zero() {
            if let Some(map) = map.get_mut(&Profile::Default) {
                map.insert("secret_key".into(), self.secret_key.key.master().into());
            }
        }

        Ok(map)
    }

    fn profile(&self) -> Option<Profile> {
        Some(self.profile.clone())
    }
}

#[crate::async_trait]
impl<'r> FromRequest<'r> for &'r Config {
    type Error = std::convert::Infallible;

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        request::Outcome::Success(req.rocket().config())
    }
}

#[doc(hidden)]
pub fn pretty_print_error(error: figment::Error) {
    use figment::error::{Kind, OneOf};

    crate::log::init_default();
    error!("Rocket configuration extraction from provider failed.");
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
