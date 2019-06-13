use rocket::{self, State};
use rocket::fairing::AdHoc;
use rocket::config::{self, Config, Environment, LoggingLevel};
use rocket::http::Status;
use rocket::local::Client;

struct LocalConfig(Config);

#[get("/check_config")]
fn check_config(config: State<'_, LocalConfig>) -> Option<()> {
    let environment = match std::env::var("ROCKET_ENV") {
        Ok(name) => name,
        Err(_) => return None
    };

    let config = &config.0;
    match &*environment {
        "development" => {
            assert_eq!(config.address, "localhost".to_string());
            assert_eq!(config.port, 8000);
            assert_eq!(config.workers, 1);
            assert_eq!(config.log_level, LoggingLevel::Normal);
            assert_eq!(config.environment, config::Environment::Development);
            assert_eq!(config.extras().count(), 2);
            assert_eq!(config.get_str("hi"), Ok("Hello!"));
            assert_eq!(config.get_bool("is_extra"), Ok(true));
        }
        "staging" => {
            assert_eq!(config.address, "0.0.0.0".to_string());
            assert_eq!(config.port, 8000);
            assert_eq!(config.workers, 8);
            assert_eq!(config.log_level, LoggingLevel::Normal);
            assert_eq!(config.environment, config::Environment::Staging);
            assert_eq!(config.extras().count(), 0);
        }
        "production" => {
            assert_eq!(config.address, "0.0.0.0".to_string());
            assert_eq!(config.port, 8000);
            assert_eq!(config.workers, 12);
            assert_eq!(config.log_level, LoggingLevel::Critical);
            assert_eq!(config.environment, config::Environment::Production);
            assert_eq!(config.extras().count(), 0);
        }
        _ => {
            panic!("Unknown environment in envvar: {}", environment);
        }
    }

    Some(())
}

pub fn test_config(environment: Environment) {
    // Manually set the config environment variable. Rocket will initialize the
    // environment in `ignite()`. We'll read this back in the handler to config.
    std::env::set_var("ROCKET_ENV", environment.to_string());

    let rocket = rocket::ignite()
        .attach(AdHoc::on_attach("Local Config", |rocket| {
            println!("Attaching local config.");
            let config = rocket.config().clone();
            Ok(rocket.manage(LocalConfig(config)))
        }))
        .mount("/", routes![check_config]);

    let client = Client::new(rocket).unwrap();
    let response = client.get("/check_config").dispatch();
    assert_eq!(response.status(), Status::Ok);
}
