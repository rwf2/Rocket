#![feature(plugin, custom_derive, custom_attribute)]
#![plugin(rocket_codegen)]

#[macro_use] extern crate rocket;
extern crate serde_json;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate rocket_contrib;
extern crate postgres;

mod users;

use std::ops::Deref;
use std::env;

use postgres::Connection as PgConnection;
use rocket::config::{self, ConfigError};
use rocket_contrib::JSON;
use rocket_contrib::database::postgres::Connection;

// use users::User;

// #[post("/", data = "<user>")]
// fn new(user: JSON<User>, conn: PgConnection) -> Result<String, String> {
//     let user = user.into_inner();
//     if user.insert(&conn) {
//         Ok("User is created.".into())
//     } else {
//         Err("Couldn't create a user.".into())
//     }
// }

// #[delete("/<id>")]
// fn delete(id: i32, conn: PgConnection) -> Result<String, String> {
//     if User::delete_with_id(id, &conn) {
//         Ok("User is deleted.".into())
//     } else {
//         Err("Couldn't delete a user.".into())
//     }
// }

#[get("/")]
fn get_all_users(conn: Connection) -> String {
    all_users(&conn)
}

fn all_users(conn: &PgConnection) -> String {
    let rows = conn.query("SELECT * FROM users", &[]).unwrap();
    let users: Vec<_> = rows.iter().map(|row| {
        let username: String = row.get(1);
        let password: String = row.get(2);
        format!("{}: {}", username, password)
    }).collect();

    users.join("; ")
}

fn main() {
    // let _ = Connection::init();
    rocket::ignite()
        .mount("/", routes![get_all_users])
        .launch();
}
