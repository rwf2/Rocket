#![feature(plugin, decl_macro, custom_derive, const_fn)]
#![plugin(rocket_codegen)]
#![recursion_limit = "128"]
#![feature(custom_attribute)]

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_derives;
extern crate r2d2;
extern crate r2d2_diesel;
extern crate r2d2_redis;
extern crate r2d2_sqlite;
extern crate redis;
extern crate rocket;
extern crate rocket_contrib;

use std::env::current_dir;

use rocket::fairing::AdHoc;
use rocket_contrib::conn::Conn;

use r2d2_sqlite::SqliteConnectionManager;
use r2d2_diesel::ConnectionManager;
use r2d2_redis::RedisConnectionManager;
use diesel::pg::PgConnection;

pub mod schema;
pub mod model;
pub mod controllers;

fn main() {
    rocket::ignite()
        .attach(AdHoc::on_attach(|rocket| {
            let default_sqlite_url = "db_sqlite/sample.sqlite".to_string();
            let default_postgres_url = "postgres://user:password@hostname/rocket_development".to_string();
            let default_redis_url = "redis://localhost".to_string();
            let max_size = 4;

            let config = rocket.config().clone();
            let sqlite_url = config
                .get_string("sqlite_url")
                .unwrap_or(default_sqlite_url);
            let postgres_url = config
                .get_string("postgres_url")
                .unwrap_or(default_postgres_url);
            let redis_url = config.get_string("redis_url").unwrap_or(default_redis_url);

            // Sqlite
            let current_dir = current_dir().unwrap();
            let sqlite_db_path = current_dir.join(sqlite_url);
            let sqlite_manager = SqliteConnectionManager::file(sqlite_db_path);
            let sqlite_pool = Conn::init_pool(sqlite_manager, max_size).unwrap();

            // Diesel PostgreSQL
            let pg_manager = ConnectionManager::<PgConnection>::new(postgres_url);
            let diesel_pool = Conn::init_pool(pg_manager, max_size).unwrap();

            // Redis
            let redis_manager = RedisConnectionManager::new(&redis_url[..]).unwrap();
            let redis_pool = Conn::init_pool(redis_manager, max_size).unwrap();
            Ok(rocket
                .manage(sqlite_pool)
                .manage(diesel_pool)
                .manage(redis_pool))
        }))
        .mount(
            "/",
            routes![
                controllers::sqlite_example,
                controllers::diesel_example,
                controllers::redis_example
            ],
        )
        .launch();
}
