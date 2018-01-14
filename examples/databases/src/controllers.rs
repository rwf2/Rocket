use std::ops::Deref;
use diesel::prelude::*;
use diesel::PgConnection;
use r2d2_sqlite::SqliteConnectionManager;
use r2d2_diesel::ConnectionManager;
use r2d2_redis::RedisConnectionManager;
use rocket_contrib::conn::Conn;
use model::{Person, User};
use schema::users::dsl::users;
use schema::users::columns::{id, username};
use redis;

#[get("/sqlite_example")]
pub fn sqlite_example(conn: Conn<SqliteConnectionManager>) -> String {
    let mut stmt = conn.prepare("SELECT id, name FROM person LIMIT 1").unwrap();
    let person_iter = stmt.query_map(&[], |row| Person {
        id: row.get(0),
        name: row.get(1),
    }).unwrap();
    let person = &person_iter.last().unwrap().unwrap();
    format!("Hello user: {} with id: {}", person.name, person.id)
}

#[get("/diesel_example")]
pub fn diesel_example(conn: Conn<ConnectionManager<PgConnection>>) -> String {
    let selected_user = users
        .select((id, username))
        .order(id.asc())
        .first::<User>(&**conn.deref())
        .optional()
        .expect("Failed to load user");
    let user = selected_user.unwrap();
    format!("Hello user: {} with id: {}", user.username, user.id)
}

#[get("/redis_example")]
pub fn redis_example(conn: Conn<RedisConnectionManager>) -> String {
    let reply = redis::cmd("PING").query::<String>(&**conn.deref()).unwrap();
    format!("Redis query result: {}", reply)
}
