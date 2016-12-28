extern crate rocket;
extern crate config as lib;
use std;
use rocket::config::{self, Environment};
use rocket::http::Method;
use rocket::LoggingLevel;
use rocket::testing::MockRequest;


pub fn test_config(environment: Environment) {
    // Manually set the config environment variable so that Rocket initializes it in `init()`.
    std::env::set_var("ROCKET_ENV", environment.to_string());
    rocket::ignite().mount("/hello", routes![lib::hello]);

    let config = config::active().unwrap();
    match environment {
        Environment::Development => {
            assert_eq!(config.address, "localhost".to_string());
            assert_eq!(config.port, 8000);
            assert_eq!(config.log_level, LoggingLevel::Normal);
            assert_eq!(config.env, config::Environment::Development);
            assert_eq!(config.extras().count(), 2);
            assert_eq!(config.get_str("hi"), Ok("Hello!"));
            assert_eq!(config.get_bool("is_extra"), Ok(true));
        }
        Environment::Staging => {
            assert_eq!(config.address, "0.0.0.0".to_string());
            assert_eq!(config.port, 80);
            assert_eq!(config.log_level, LoggingLevel::Normal);
            assert_eq!(config.env, config::Environment::Staging);
            assert_eq!(config.extras().count(), 0);
        }
        Environment::Production => {
            assert_eq!(config.address, "0.0.0.0".to_string());
            assert_eq!(config.port, 80);
            assert_eq!(config.log_level, LoggingLevel::Critical);
            assert_eq!(config.env, config::Environment::Production);
            assert_eq!(config.extras().count(), 0);
        }
    }

    // Rocket `take`s the key, so this should always be `None`
    assert_eq!(config.take_session_key(), None);
}

pub fn test_hello() {
    let rocket = rocket::ignite().mount("/hello", routes![lib::hello]);
    let mut request = MockRequest::new(Method::Get, "/hello");
    let mut response = request.dispatch_with(&rocket);

    assert_eq!(response.body().and_then(|b| b.into_string()),
               Some("Hello, world!".to_string()));
}
