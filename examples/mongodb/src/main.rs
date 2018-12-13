#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
#[macro_use] extern crate rocket_contrib;
#[macro_use] extern crate serde_derive;

mod api;
mod db;
mod models;

fn main() {
    rocket::ignite()
        .attach(db::MyDatabase::fairing())
        .mount("/todos", routes![
            api::get_todos,
            api::get_todo,
            api::create_todo,
            api::update_todo,
            api::delete_todo,
        ])
        .launch();
}
