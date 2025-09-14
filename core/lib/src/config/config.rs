use figment::{Figment, Profile, Provider, Metadata, error::Result};
use figment::providers::{Serialized, Env, Toml, Format};
use figment::value::{Map, Dict, magic::RelativePathBuf};
use serde::{Deserialize, Serialize};

#[cfg(feature = "secrets")]
use crate::config::SecretKey;
use crate::config::{ShutdownConfig, Level, TraceFormat, Ident, CliColors};
use crate::request::{self, Request, FromRequest};
use crate::http::uncased::Uncased;
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
/// of `log_level` and `log_format`, which are `info` / `pretty` in `debug` and
/// `error` / `compact` in `release`, and `secret_key`, which is regenerated
/// from a random value if not set in "debug" mode only, all default values are
/// identical in all profiles.
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
///     [`Source`](figment::Source) and uses default interpolation.
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
    /// _**Note:** This field is never serialized nor deserialized. When a
    /// `Config` is merged into a `Figment` as a `Provider`, this profile is
    /// selected on the `Figment`. When a `Config` is extracted, this field is
    /// set to the extracting Figment's selected `Profile`._
    #[serde(skip)]
    pub profile: Profile,
    /// Number of threads to use for executing futures. **(default: `num_cores`)**
    ///
    /// _**Note:** Rocket only reads this value from sources in the [default
    /// provider](Config::figment())._
    pub workers: usize,
    /// Limit on threads to start for blocking tasks. **(default: `512`)**
    pub max_blocking: usize,
    /// How, if at all, to identify the server via the `Server` header.
    /// **(default: `"Rocket"`)**
    pub ident: Ident,
    /// The name of a header, whose value is typically set by an intermediary
    /// server or proxy, which contains the real IP address of the connecting
    /// client. Used internally and by [`Request::client_ip()`] and
    /// [`Request::real_ip()`].
    ///
    /// To disable using any header for this purpose, set this value to `false`
    /// or `None`. Deserialization semantics are identical to those of [`Ident`]
    /// except that the value must syntactically be a valid HTTP header name.
    ///
    /// **(default: `"X-Real-IP"`)**
    #[serde(deserialize_with = "crate::config::http_header::deserialize")]
    pub ip_header: Option<Uncased<'static>>,
    /// The name of a header, whose value is typically set by an intermediary
    /// server or proxy, which contains the protocol ("http" or "https") used by
    /// the connecting client. This is usually [`"X-Forwarded-Proto"`], as that
    /// is the de-facto standard.
    ///
    /// The header value is parsed into a [`ProxyProto`], accessible via
    /// [`Request::proxy_proto()`]. The value influences
    /// [`Request::context_is_likely_secure()`] and the default value for the
    /// `Secure` flag in cookies added to [`CookieJar`]s.
    ///
    /// To disable using any header for this purpose, set this value to `false`
    /// or `None`. Deserialization semantics are identical to those of
    /// [`Config::ip_header`] (the value must be a valid HTTP header name).
    ///
    /// **(default: `None`)**
    ///
    /// [`CookieJar`]: crate::http::CookieJar
    /// [`ProxyProto`]: crate::http::ProxyProto
    /// [`"X-Forwarded-Proto"`]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-Forwarded-Proto
    #[serde(deserialize_with = "crate::config::http_header::deserialize")]
    pub proxy_proto_header: Option<Uncased<'static>>,
    /// Streaming read size limits. **(default: [`Limits::default()`])**
    pub limits: Limits,
    /// Directory to store temporary files in. **(default:
    /// [`std::env::temp_dir()`])**
    #[serde(serialize_with = "RelativePathBuf::serialize_relative")]
    pub temp_dir: RelativePathBuf,
    /// Keep-alive timeout in seconds; disabled when `0`. **(default: `5`)**
    pub keep_alive: u32,
    /// The secret key for signing and encrypting. **(default: `0`)**
    ///
    /// _**Note:** This field _always_ serializes as a 256-bit array of `0`s to
    /// aid in preventing leakage of the secret key._
    #[cfg(feature = "secrets")]
    #[cfg_attr(nightly, doc(cfg(feature = "secrets")))]
    #[serde(serialize_with = "SecretKey::serialize_zero")]
    pub secret_key: SecretKey,
    /// Graceful shutdown configuration. **(default: [`ShutdownConfig::default()`])**
    pub shutdown: ShutdownConfig,
    /// Max level to log. **(default: _debug_ `info` / _release_ `error`)**
    #[serde(with = "crate::trace::level")]
    pub log_level: Option<Level>,
    /// Format to use when logging. **(default: _debug_ `pretty` / _release_ `compact`)**
    pub log_format: TraceFormat,
    /// Whether to use colors and emoji when logging. **(default:
    /// [`CliColors::Auto`])**
    pub cli_colors: CliColors,
    /// PRIVATE: This structure may grow (but never change otherwise) in a
    /// non-breaking release. As such, constructing this structure should
    /// _always_ be done using a public constructor or update syntax:
    ///
    /// ```rust
    /// use rocket::Config;
    ///
    /// let config = Config {
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
            workers: num_cpus::get(),
            max_blocking: 512,
            ident: Ident::default(),
            ip_header: Some(Uncased::from_borrowed("X-Real-IP")),
            proxy_proto_header: None,
            limits: Limits::default(),
            temp_dir: std::env::temp_dir().into(),
            keep_alive: 5,
            #[cfg(feature = "secrets")]
            secret_key: SecretKey::zero(),
            shutdown: ShutdownConfig::default(),
            log_level: Some(Level::INFO),
            log_format: TraceFormat::Pretty,
            cli_colors: CliColors::Auto,
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
            log_level: Some(Level::ERROR),
            log_format: TraceFormat::Compact,
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
    /// If extraction fails, logs an error message indicating the error and
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
        use crate::trace::Trace;

        Self::try_from(provider).unwrap_or_else(|e| {
            e.trace_error();
            panic!("aborting due to configuration error(s)")
        })
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
    /// The stringy parameter name for setting/extracting [`Config::workers`].
    pub const WORKERS: &'static str = "workers";

    /// The stringy parameter name for setting/extracting [`Config::max_blocking`].
    pub const MAX_BLOCKING: &'static str = "max_blocking";

    /// The stringy parameter name for setting/extracting [`Config::keep_alive`].
    pub const KEEP_ALIVE: &'static str = "keep_alive";

    /// The stringy parameter name for setting/extracting [`Config::ident`].
    pub const IDENT: &'static str = "ident";

    /// The stringy parameter name for setting/extracting [`Config::ip_header`].
    pub const IP_HEADER: &'static str = "ip_header";

    /// The stringy parameter name for setting/extracting [`Config::proxy_proto_header`].
    pub const PROXY_PROTO_HEADER: &'static str = "proxy_proto_header";

    /// The stringy parameter name for setting/extracting [`Config::limits`].
    pub const LIMITS: &'static str = "limits";

    /// The stringy parameter name for setting/extracting [`Config::secret_key`].
    pub const SECRET_KEY: &'static str = "secret_key";

    /// The stringy parameter name for setting/extracting [`Config::temp_dir`].
    pub const TEMP_DIR: &'static str = "temp_dir";

    /// The stringy parameter name for setting/extracting [`Config::log_level`].
    pub const LOG_LEVEL: &'static str = "log_level";

    /// The stringy parameter name for setting/extracting [`Config::log_format`].
    pub const LOG_FORMAT: &'static str = "log_format";

    /// The stringy parameter name for setting/extracting [`Config::shutdown`].
    pub const SHUTDOWN: &'static str = "shutdown";

    /// The stringy parameter name for setting/extracting [`Config::cli_colors`].
    pub const CLI_COLORS: &'static str = "cli_colors";

    /// An array of all of the stringy parameter names.
    pub const PARAMETERS: &'static [&'static str] = &[
        Self::WORKERS, Self::MAX_BLOCKING, Self::KEEP_ALIVE, Self::IDENT,
        Self::IP_HEADER, Self::PROXY_PROTO_HEADER, Self::LIMITS,
        Self::SECRET_KEY, Self::TEMP_DIR, Self::LOG_LEVEL, Self::LOG_FORMAT,
        Self::SHUTDOWN, Self::CLI_COLORS,
    ];

    /// The stringy parameter name for setting/extracting [`Config::profile`].
    ///
    /// This isn't `pub` because setting it directly does nothing.
    const PROFILE: &'static str = "profile";

    /// An array of deprecated stringy parameter names.
    pub(crate) const DEPRECATED_KEYS: &'static [(&'static str, Option<&'static str>)] = &[
        ("env", Some(Self::PROFILE)), ("log", Some(Self::LOG_LEVEL)),
        ("read_timeout", None), ("write_timeout", None),
    ];

    /// Secret keys that have been used in docs or leaked otherwise.
    #[cfg(feature = "secrets")]
    pub(crate) const KNOWN_SECRET_KEYS: &'static [&'static str] = &[
        "hPRYyVRiMyxpw5sBB1XeCMN1kFsDCqKvBi2QJxBVHQk="
    ];
}

impl Provider for Config {
    #[track_caller]
    fn metadata(&self) -> Metadata {
        if self == &Config::default() {
            Metadata::named("rocket::Config::default()")
        } else {
            Metadata::named("rocket::Config")
        }
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
    type Forward = std::convert::Infallible;
    type Error = std::convert::Infallible;

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error, Self::Forward> {
        request::Outcome::Success(req.rocket().config())
    }
}
