#![feature(plugin, decl_macro, custom_derive, const_fn)]
#![plugin(rocket_codegen)]
#![recursion_limit="128"]
#![feature(custom_attribute)]

extern crate rocket;
extern crate rocket_contrib;
extern crate r2d2;
extern crate r2d2_sqlite;
extern crate r2d2_diesel;
#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_derives;

use std::env::current_dir;
use rocket_contrib::conn::init_pool;
use r2d2_sqlite::SqliteConnectionManager;
use r2d2_diesel::ConnectionManager;
use diesel::pg::PgConnection;
use rocket::config::{Config, Environment};

pub mod schema;
pub mod model;
pub mod controllers;

fn main() {
    let default_sqlite_url = "db_sqlite/sample.sqlite".to_string();
    let default_postgres_url = "postgres://user:password@hostname/rocket_development".to_string();
    let max_size = 4;
    let env = Environment::active().unwrap();
    let config = Config::new(env).unwrap();
    let sqlite_url = config.get_string("sqlite_url").unwrap_or(default_sqlite_url);
    let postgres_url = config.get_string("postgres_url").unwrap_or(default_postgres_url);

    // Sqlite
    let current_dir = current_dir().unwrap();
    let sqlite_db_path = current_dir.join(sqlite_url);
    let sqlite_manager = SqliteConnectionManager::file(sqlite_db_path);
    let sqlite_pool = init_pool(sqlite_manager, max_size)
        .unwrap();

    // Diesel Postgresql
    let pg_manager = ConnectionManager::<PgConnection>::new(postgres_url);
    let diesel_pool = init_pool(pg_manager, max_size)
        .unwrap();

    rocket::ignite()
        .manage(sqlite_pool)
        .manage(diesel_pool)
        .mount("/", routes![controllers::sqlite_example, controllers::diesel_example])
        .launch();
}
