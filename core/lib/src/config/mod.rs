//! Server and application configuration.
//!
//! See the [configuration guide] for full details.
//!
//! [configuration guide]: https://rocket.rs/master/guide/configuration/
//!
//! ## Extracting Configuration Parameters
//!
//! Rocket exposes the active [`Figment`] via [`Rocket::figment()`] and
//! [`Rocket::figment()`]. Any value that implements [`Deserialize`] can be
//! extracted from the figment:
//!
//! ```rust
//! use rocket::fairing::AdHoc;
//!
//! #[derive(serde::Deserialize)]
//! struct AppConfig {
//!     id: Option<usize>,
//!     port: u16,
//! }
//!
//! #[rocket::launch]
//! fn rocket() -> _ {
//!     rocket::ignite().attach(AdHoc::config::<AppConfig>())
//! }
//! ```
//!
//! [`Figment`]: figment::Figment
//! [`Rocket::figment()`]: crate::Rocket::figment()
//! [`Rocket::figment()`]: crate::Rocket::figment()
//! [`Deserialize`]: serde::Deserialize
//!
//! ## Custom Providers
//!
//! A custom provider can be set via [`rocket::custom()`], which replaces calls to
//! [`rocket::ignite()`]. The configured provider can be built on top of
//! [`Config::figment()`], [`Config::default()`], both, or neither. The
//! [Figment](@figment) documentation has full details on instantiating existing
//! providers like [`Toml`]() and [`Env`] as well as creating custom providers for
//! more complex cases.
//!
//! Configuration values can be overridden at runtime by merging figment's tuple
//! providers with Rocket's default provider:
//!
//! ```rust
//! # #[macro_use] extern crate rocket;
//! use rocket::data::{Limits, ToByteUnit};
//!
//! #[launch]
//! fn rocket() -> _ {
//!     let figment = rocket::Config::figment()
//!         .merge(("port", 1111))
//!         .merge(("limits", Limits::new().limit("json", 2.mebibytes())));
//!
//!     rocket::custom(figment).mount("/", routes![/* .. */])
//! }
//! ```
//!
//! An application that wants to use Rocket's defaults for [`Config`], but not
//! its configuration sources, while allowing the application to be configured
//! via an `App.toml` file and `APP_` environment variables, can be structured
//! as follows:
//!
//! ```rust
//! # #[macro_use] extern crate rocket;
//! use serde::{Serialize, Deserialize};
//! use figment::{Figment, providers::{Format, Toml, Serialized, Env}};
//! use rocket::fairing::AdHoc;
//!
//! #[derive(Debug, Deserialize, Serialize)]
//! struct Config {
//!     app_value: usize,
//!     /* and so on.. */
//! }
//!
//! impl Default for Config {
//!     fn default() -> Config {
//!         Config { app_value: 3, }
//!     }
//! }
//!
//! #[launch]
//! fn rocket() -> _ {
//!     let figment = Figment::from(rocket::Config::default())
//!         .merge(Serialized::defaults(Config::default()))
//!         .merge(Toml::file("App.toml"))
//!         .merge(Env::prefixed("APP_"));
//!
//!     rocket::custom(figment)
//!         .mount("/", routes![/* .. */])
//!         .attach(AdHoc::config::<Config>())
//! }
//! ```
//!
//! If the application's config should behave like the configuration of Rocket,
//! i.e. use the same [`Profile`]s and behaviour, you need to instead merge them as follows:
//!
//! ```rust,ignore
//! let figment = Figment::from(rocket::Config::default())
//!     .merge(Serialized::defaults(Config::default()))
//!     .merge(Toml::file(Env::var_or("APP_CONFIG", "App.toml")).nested())
//!     .merge(Env::prefixed("APP_").global());
//! ```
//!
//! [`rocket::custom()`]: crate::custom()
//! [`rocket::ignite()`]: crate::ignite()
//! [`Toml`]: figment::providers::Toml
//! [`Env`]: figment::providers::Env
//! [`Profile`]: https://docs.rs/figment/0.9/figment/struct.Profile.html

mod secret_key;
mod config;
mod tls;

#[doc(hidden)] pub use config::pretty_print_error;

pub use config::Config;
pub use crate::logger::LogLevel;
pub use secret_key::SecretKey;
pub use tls::TlsConfig;

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;
    use figment::Figment;

    use crate::config::{Config, TlsConfig};
    use crate::logger::LogLevel;
    use crate::data::{Limits, ToByteUnit};

    #[test]
    fn test_default_round_trip() {
        let figment = Figment::from(Config::default());

        assert_eq!(figment.profile(), Config::DEFAULT_PROFILE);

        #[cfg(debug_assertions)]
        assert_eq!(figment.profile(), Config::DEBUG_PROFILE);

        #[cfg(not(debug_assertions))]
        assert_eq!(figment.profile(), Config::RELEASE_PROFILE);

        let config: Config = figment.extract().unwrap();
        assert_eq!(config, Config::default());

        #[cfg(debug_assertions)]
        assert_eq!(config, Config::debug_default());

        #[cfg(not(debug_assertions))]
        assert_eq!(config, Config::release_default());

        assert_eq!(Config::from(Config::default()), Config::default());
    }

    #[test]
    fn test_profile_env() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("ROCKET_PROFILE", "debug");
            let figment = Figment::from(Config::default());
            assert_eq!(figment.profile(), "debug");

            jail.set_env("ROCKET_PROFILE", "release");
            let figment = Figment::from(Config::default());
            assert_eq!(figment.profile(), "release");

            jail.set_env("ROCKET_PROFILE", "random");
            let figment = Figment::from(Config::default());
            assert_eq!(figment.profile(), "random");

            Ok(())
        });
    }

    #[test]
    fn test_toml_file() {
        figment::Jail::expect_with(|jail| {
            jail.create_file("Rocket.toml", r#"
                [default]
                address = "1.2.3.4"
                port = 1234
                workers = 20
                keep_alive = 10
                log_level = "off"
                cli_colors = 0
            "#)?;

            let config = Config::from(Config::figment());
            assert_eq!(config, Config {
                address: Ipv4Addr::new(1, 2, 3, 4).into(),
                port: 1234,
                workers: 20,
                keep_alive: 10,
                log_level: LogLevel::Off,
                cli_colors: false,
                ..Config::default()
            });

            jail.create_file("Rocket.toml", r#"
                [global]
                address = "1.2.3.4"
                port = 1234
                workers = 20
                keep_alive = 10
                log_level = "off"
                cli_colors = 0
            "#)?;

            let config = Config::from(Config::figment());
            assert_eq!(config, Config {
                address: Ipv4Addr::new(1, 2, 3, 4).into(),
                port: 1234,
                workers: 20,
                keep_alive: 10,
                log_level: LogLevel::Off,
                cli_colors: false,
                ..Config::default()
            });

            jail.create_file("Rocket.toml", r#"
                [global]
                ctrlc = 0

                [global.tls]
                certs = "/ssl/cert.pem"
                key = "/ssl/key.pem"

                [global.limits]
                forms = "1mib"
                json = "10mib"
                stream = "50kib"
            "#)?;

            let config = Config::from(Config::figment());
            assert_eq!(config, Config {
                ctrlc: false,
                tls: Some(TlsConfig::from_paths("/ssl/cert.pem", "/ssl/key.pem")),
                limits: Limits::default()
                    .limit("forms", 1.mebibytes())
                    .limit("json", 10.mebibytes())
                    .limit("stream", 50.kibibytes()),
                ..Config::default()
            });

            jail.create_file("Rocket.toml", r#"
                [global.tls]
                certs = "cert.pem"
                key = "key.pem"
            "#)?;

            let config = Config::from(Config::figment());
            assert_eq!(config, Config {
                tls: Some(TlsConfig::from_paths(
                    jail.directory().join("cert.pem"), jail.directory().join("key.pem")
                )),
                ..Config::default()
            });

            jail.set_env("ROCKET_CONFIG", "Other.toml");
            jail.create_file("Other.toml", r#"
                [default]
                address = "1.2.3.4"
                port = 1234
                workers = 20
                keep_alive = 10
                log_level = "off"
                cli_colors = 0
            "#)?;

            let config = Config::from(Config::figment());
            assert_eq!(config, Config {
                address: Ipv4Addr::new(1, 2, 3, 4).into(),
                port: 1234,
                workers: 20,
                keep_alive: 10,
                log_level: LogLevel::Off,
                cli_colors: false,
                ..Config::default()
            });

            Ok(())
        });
    }

    #[test]
    fn test_profiles_merge() {
        figment::Jail::expect_with(|jail| {
            jail.create_file("Rocket.toml", r#"
                [default.limits]
                stream = "50kb"

                [global]
                limits = { forms = "2kb" }

                [debug.limits]
                file = "100kb"
            "#)?;

            jail.set_env("ROCKET_PROFILE", "unknown");
            let config = Config::from(Config::figment());
            assert_eq!(config, Config {
                limits: Limits::default()
                    .limit("stream", 50.kilobytes())
                    .limit("forms", 2.kilobytes()),
                ..Config::default()
            });

            jail.set_env("ROCKET_PROFILE", "debug");
            let config = Config::from(Config::figment());
            assert_eq!(config, Config {
                limits: Limits::default()
                    .limit("stream", 50.kilobytes())
                    .limit("forms", 2.kilobytes())
                    .limit("file", 100.kilobytes()),
                ..Config::default()
            });

            Ok(())
        });
    }

    #[test]
    fn test_env_vars_merge() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("ROCKET_PORT", 9999);
            let config = Config::from(Config::figment());
            assert_eq!(config, Config {
                port: 9999,
                ..Config::default()
            });

            jail.set_env("ROCKET_TLS", r#"{certs="certs.pem"}"#);
            let first_figment = Config::figment();
            jail.set_env("ROCKET_TLS", r#"{key="key.pem"}"#);
            let prev_figment = Config::figment().join(&first_figment);
            let config = Config::from(&prev_figment);
            assert_eq!(config, Config {
                port: 9999,
                tls: Some(TlsConfig::from_paths("certs.pem", "key.pem")),
                ..Config::default()
            });

            jail.set_env("ROCKET_TLS", r#"{certs="new.pem"}"#);
            let config = Config::from(Config::figment().join(&prev_figment));
            assert_eq!(config, Config {
                port: 9999,
                tls: Some(TlsConfig::from_paths("new.pem", "key.pem")),
                ..Config::default()
            });

            jail.set_env("ROCKET_LIMITS", r#"{stream=100kiB}"#);
            let config = Config::from(Config::figment().join(&prev_figment));
            assert_eq!(config, Config {
                port: 9999,
                tls: Some(TlsConfig::from_paths("new.pem", "key.pem")),
                limits: Limits::default().limit("stream", 100.kibibytes()),
                ..Config::default()
            });

            Ok(())
        });
    }

    #[test]
    fn test_precedence() {
        figment::Jail::expect_with(|jail| {
            jail.create_file("Rocket.toml", r#"
                [global.limits]
                forms = "1mib"
                stream = "50kb"
                file = "100kb"
            "#)?;

            let config = Config::from(Config::figment());
            assert_eq!(config, Config {
                limits: Limits::default()
                    .limit("forms", 1.mebibytes())
                    .limit("stream", 50.kilobytes())
                    .limit("file", 100.kilobytes()),
                ..Config::default()
            });

            jail.set_env("ROCKET_LIMITS", r#"{stream=3MiB,capture=2MiB}"#);
            let config = Config::from(Config::figment());
            assert_eq!(config, Config {
                limits: Limits::default()
                    .limit("file", 100.kilobytes())
                    .limit("forms", 1.mebibytes())
                    .limit("stream", 3.mebibytes())
                    .limit("capture", 2.mebibytes()),
                ..Config::default()
            });

            jail.set_env("ROCKET_PROFILE", "foo");
            let val: Result<String, _> = Config::figment().extract_inner("profile");
            assert!(val.is_err());

            Ok(())
        });
    }
}
