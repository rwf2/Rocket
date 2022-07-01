#[macro_use] extern crate rocket;

#[cfg(test)] mod tests;

use rocket::{State, Config};
use rocket::fairing::AdHoc;
use rocket::serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
struct AppConfig {
    key: String,
    port: u16
}

#[get("/")]
fn read_config(rocket_config: &Config, app_config: &State<AppConfig>) -> String {
    format!("{:#?}\n{:#?}", app_config, rocket_config)
}

// See Rocket.toml file. Running this server will print the config. Try running
// with `ROCKET_PROFILE=release` manually by setting the environment variable
// and automatically by compiling with `--release`.
#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![read_config])
        .attach(AdHoc::config::<AppConfig>())
}
