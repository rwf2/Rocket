use rocket::config::{Config, LogLevel, BindableAddr};
use std::net::{Ipv4Addr, SocketAddr};

async fn test_config(profile: &str) {
    let provider = Config::figment().select(profile);
    let rocket = rocket::custom(provider).ignite().await.unwrap();
    let config = rocket.config();
    match &*profile {
        "debug" => {
            assert_eq!(config.address, BindableAddr::Tcp(
                SocketAddr::new(std::net::Ipv4Addr::LOCALHOST.into(), 8000)
            );
            assert_eq!(config.workers, 1);
            assert_eq!(config.keep_alive, 0);
            assert_eq!(config.log_level, LogLevel::Normal);
        }
        "release" => {
            assert_eq!(config.address, BindableAddr::Tcp(
                SocketAddr::new(std::net::Ipv4Addr::LOCALHOST.into(), 8000)
            );
            assert_eq!(config.workers, 12);
            assert_eq!(config.keep_alive, 5);
            assert_eq!(config.log_level, LogLevel::Critical);
            assert!(!config.secret_key.is_zero());
        }
        _ => {
            panic!("Unknown profile: {}", profile);
        }
    }
}

#[async_test]
async fn test_debug_config() {
    test_config("debug").await;
}

#[async_test]
async fn test_release_config() {
    test_config("release").await;
}
