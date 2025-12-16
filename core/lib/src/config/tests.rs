use figment::{Figment, Profile};
use pretty_assertions::assert_eq;
use tracing::Level;

// use crate::log::LogLevel;
use crate::config::{CliColors, Config};
use crate::data::{Limits, ToByteUnit};

#[test]
fn test_figment_is_default() {
    figment::Jail::expect_with(|_| {
        let mut default: Config = Config::figment().extract().unwrap();
        default.profile = Config::default().profile;
        assert_eq!(default, Config::default());
        Ok(())
    });
}

#[test]
fn test_default_round_trip() {
    figment::Jail::expect_with(|_| {
        let original = Config::figment();
        let roundtrip = Figment::from(Config::from(&original));
        for figment in &[original, roundtrip] {
            let config = Config::from(figment);
            assert_eq!(config, Config::default());
        }

        Ok(())
    });
}

#[test]
fn test_profile_env() {
    figment::Jail::expect_with(|jail| {
        jail.set_env("ROCKET_PROFILE", "debug");
        let figment = Config::figment();
        assert_eq!(figment.profile(), "debug");

        jail.set_env("ROCKET_PROFILE", "release");
        let figment = Config::figment();
        assert_eq!(figment.profile(), "release");

        jail.set_env("ROCKET_PROFILE", "random");
        let figment = Config::figment();
        assert_eq!(figment.profile(), "random");

        Ok(())
    });
}

#[test]
fn test_toml_file() {
    figment::Jail::expect_with(|jail| {
        jail.create_file(
            "Rocket.toml",
            r#"
                [default]
                ident = "Something Cool"
                workers = 20
                keep_alive = 10
                log_level = "off"
                cli_colors = 0
            "#,
        )?;

        let config = Config::from(Config::figment());
        assert_eq!(
            config,
            Config {
                workers: 20,
                ident: ident!("Something Cool"),
                keep_alive: 10,
                log_level: None,
                cli_colors: CliColors::Never,
                ..Config::default()
            }
        );

        jail.create_file(
            "Rocket.toml",
            r#"
                [global]
                ident = "Something Else Cool"
                workers = 20
                keep_alive = 10
                log_level = "trace"
                cli_colors = 0
            "#,
        )?;

        let config = Config::from(Config::figment());
        assert_eq!(
            config,
            Config {
                workers: 20,
                ident: ident!("Something Else Cool"),
                keep_alive: 10,
                log_level: Some(Level::TRACE),
                cli_colors: CliColors::Never,
                ..Config::default()
            }
        );

        jail.set_env("ROCKET_CONFIG", "Other.toml");
        jail.create_file(
            "Other.toml",
            r#"
                [default]
                workers = 20
                keep_alive = 10
                log_level = 2
                cli_colors = 0
            "#,
        )?;

        let config = Config::from(Config::figment());
        assert_eq!(
            config,
            Config {
                workers: 20,
                keep_alive: 10,
                log_level: Some(Level::WARN),
                cli_colors: CliColors::Never,
                ..Config::default()
            }
        );

        Ok(())
    });
}

#[test]
fn test_cli_colors() {
    figment::Jail::expect_with(|jail| {
        jail.create_file(
            "Rocket.toml",
            r#"
                [default]
                cli_colors = "never"
            "#,
        )?;

        let config = Config::from(Config::figment());
        assert_eq!(config.cli_colors, CliColors::Never);

        jail.create_file(
            "Rocket.toml",
            r#"
                [default]
                cli_colors = "auto"
            "#,
        )?;

        let config = Config::from(Config::figment());
        assert_eq!(config.cli_colors, CliColors::Auto);

        jail.create_file(
            "Rocket.toml",
            r#"
                [default]
                cli_colors = "always"
            "#,
        )?;

        let config = Config::from(Config::figment());
        assert_eq!(config.cli_colors, CliColors::Always);

        jail.create_file(
            "Rocket.toml",
            r#"
                [default]
                cli_colors = true
            "#,
        )?;

        let config = Config::from(Config::figment());
        assert_eq!(config.cli_colors, CliColors::Auto);

        jail.create_file(
            "Rocket.toml",
            r#"
                [default]
                cli_colors = false
            "#,
        )?;

        let config = Config::from(Config::figment());
        assert_eq!(config.cli_colors, CliColors::Never);

        jail.create_file("Rocket.toml", r#"[default]"#)?;
        let config = Config::from(Config::figment());
        assert_eq!(config.cli_colors, CliColors::Auto);

        jail.create_file(
            "Rocket.toml",
            r#"
                [default]
                cli_colors = 1
            "#,
        )?;

        let config = Config::from(Config::figment());
        assert_eq!(config.cli_colors, CliColors::Auto);

        jail.create_file(
            "Rocket.toml",
            r#"
                [default]
                cli_colors = 0
            "#,
        )?;

        let config = Config::from(Config::figment());
        assert_eq!(config.cli_colors, CliColors::Never);

        jail.set_env("ROCKET_CLI_COLORS", 1);
        let config = Config::from(Config::figment());
        assert_eq!(config.cli_colors, CliColors::Auto);

        jail.set_env("ROCKET_CLI_COLORS", 0);
        let config = Config::from(Config::figment());
        assert_eq!(config.cli_colors, CliColors::Never);

        jail.set_env("ROCKET_CLI_COLORS", true);
        let config = Config::from(Config::figment());
        assert_eq!(config.cli_colors, CliColors::Auto);

        jail.set_env("ROCKET_CLI_COLORS", false);
        let config = Config::from(Config::figment());
        assert_eq!(config.cli_colors, CliColors::Never);

        jail.set_env("ROCKET_CLI_COLORS", "always");
        let config = Config::from(Config::figment());
        assert_eq!(config.cli_colors, CliColors::Always);

        jail.set_env("ROCKET_CLI_COLORS", "NEveR");
        let config = Config::from(Config::figment());
        assert_eq!(config.cli_colors, CliColors::Never);

        jail.set_env("ROCKET_CLI_COLORS", "auTO");
        let config = Config::from(Config::figment());
        assert_eq!(config.cli_colors, CliColors::Auto);

        Ok(())
    })
}

#[test]
fn test_profiles_merge() {
    figment::Jail::expect_with(|jail| {
        jail.create_file(
            "Rocket.toml",
            r#"
                [default.limits]
                stream = "50kb"

                [global]
                limits = { forms = "2kb" }

                [debug.limits]
                file = "100kb"
            "#,
        )?;

        jail.set_env("ROCKET_PROFILE", "unknown");
        let config = Config::from(Config::figment());
        assert_eq!(
            config,
            Config {
                profile: Profile::const_new("unknown"),
                limits: Limits::default()
                    .limit("stream", 50.kilobytes())
                    .limit("forms", 2.kilobytes()),
                ..Config::default()
            }
        );

        jail.set_env("ROCKET_PROFILE", "debug");
        let config = Config::from(Config::figment());
        assert_eq!(
            config,
            Config {
                profile: Profile::const_new("debug"),
                limits: Limits::default()
                    .limit("stream", 50.kilobytes())
                    .limit("forms", 2.kilobytes())
                    .limit("file", 100.kilobytes()),
                ..Config::default()
            }
        );

        Ok(())
    });
}

#[test]
fn test_env_vars_merge() {
    use crate::config::{Ident, ShutdownConfig};

    figment::Jail::expect_with(|jail| {
        jail.set_env("ROCKET_KEEP_ALIVE", 9999);
        let config = Config::from(Config::figment());
        assert_eq!(
            config,
            Config {
                keep_alive: 9999,
                ..Config::default()
            }
        );

        jail.set_env("ROCKET_SHUTDOWN", r#"{grace=7}"#);
        let first_figment = Config::figment();
        jail.set_env("ROCKET_SHUTDOWN", r#"{mercy=10}"#);
        let prev_figment = Config::figment().join(&first_figment);
        let config = Config::from(&prev_figment);
        assert_eq!(
            config,
            Config {
                keep_alive: 9999,
                shutdown: ShutdownConfig {
                    grace: 7,
                    mercy: 10,
                    ..Default::default()
                },
                ..Config::default()
            }
        );

        jail.set_env("ROCKET_SHUTDOWN", r#"{mercy=20}"#);
        let config = Config::from(Config::figment().join(&prev_figment));
        assert_eq!(
            config,
            Config {
                keep_alive: 9999,
                shutdown: ShutdownConfig {
                    grace: 7,
                    mercy: 20,
                    ..Default::default()
                },
                ..Config::default()
            }
        );

        jail.set_env("ROCKET_LIMITS", r#"{stream=100kiB}"#);
        let config = Config::from(Config::figment().join(&prev_figment));
        assert_eq!(
            config,
            Config {
                keep_alive: 9999,
                shutdown: ShutdownConfig {
                    grace: 7,
                    mercy: 20,
                    ..Default::default()
                },
                limits: Limits::default().limit("stream", 100.kibibytes()),
                ..Config::default()
            }
        );

        jail.set_env("ROCKET_IDENT", false);
        let config = Config::from(Config::figment().join(&prev_figment));
        assert_eq!(
            config,
            Config {
                keep_alive: 9999,
                shutdown: ShutdownConfig {
                    grace: 7,
                    mercy: 20,
                    ..Default::default()
                },
                limits: Limits::default().limit("stream", 100.kibibytes()),
                ident: Ident::none(),
                ..Config::default()
            }
        );

        Ok(())
    });
}

#[test]
fn test_precedence() {
    figment::Jail::expect_with(|jail| {
        jail.create_file(
            "Rocket.toml",
            r#"
                [global.limits]
                forms = "1mib"
                stream = "50kb"
                file = "100kb"
            "#,
        )?;

        let config = Config::from(Config::figment());
        assert_eq!(
            config,
            Config {
                limits: Limits::default()
                    .limit("forms", 1.mebibytes())
                    .limit("stream", 50.kilobytes())
                    .limit("file", 100.kilobytes()),
                ..Config::default()
            }
        );

        jail.set_env("ROCKET_LIMITS", r#"{stream=3MiB,capture=2MiB}"#);
        let config = Config::from(Config::figment());
        assert_eq!(
            config,
            Config {
                limits: Limits::default()
                    .limit("file", 100.kilobytes())
                    .limit("forms", 1.mebibytes())
                    .limit("stream", 3.mebibytes())
                    .limit("capture", 2.mebibytes()),
                ..Config::default()
            }
        );

        jail.set_env("ROCKET_PROFILE", "foo");
        let val: Result<String, _> = Config::figment().extract_inner("profile");
        assert!(val.is_err());

        Ok(())
    });
}

#[test]
#[cfg(feature = "secrets")]
fn test_err_on_non_debug_and_no_secret_key() {
    figment::Jail::expect_with(|jail| {
        jail.set_env("ROCKET_PROFILE", "release");
        let rocket = crate::custom(Config::figment());
        crate::local::blocking::Client::untracked(rocket).expect_err("release secret key");
        Ok(())
    });
}

#[test]
#[cfg(feature = "secrets")]
fn test_err_on_non_debug2_and_no_secret_key() {
    figment::Jail::expect_with(|jail| {
        jail.set_env("ROCKET_PROFILE", "boop");
        let rocket = crate::custom(Config::figment());
        crate::local::blocking::Client::tracked(rocket).expect_err("boop secret key");
        Ok(())
    });
}

#[test]
fn test_no_err_on_debug_and_no_secret_key() {
    figment::Jail::expect_with(|jail| {
        jail.set_env("ROCKET_PROFILE", "debug");
        let figment = Config::figment();
        assert!(crate::local::blocking::Client::untracked(crate::custom(&figment)).is_ok());
        crate::async_main(async {
            let rocket = crate::custom(&figment);
            assert!(crate::local::asynchronous::Client::tracked(rocket)
                .await
                .is_ok());
        });

        Ok(())
    });
}

#[test]
fn test_no_err_on_release_and_custom_secret_key() {
    figment::Jail::expect_with(|jail| {
        jail.set_env("ROCKET_PROFILE", "release");
        let key = "Bx4Gb+aSIfuoEyMHD4DvNs92+wmzfQK98qc6MiwyPY4=";
        let figment = Config::figment().merge(("secret_key", key));

        assert!(crate::local::blocking::Client::tracked(crate::custom(&figment)).is_ok());
        crate::async_main(async {
            let rocket = crate::custom(&figment);
            assert!(crate::local::asynchronous::Client::untracked(rocket)
                .await
                .is_ok());
        });

        Ok(())
    });
}
