#![feature(plugin)]
#![feature(custom_derive)]
#![plugin(rocket_codegen, diesel_codegen)]
extern crate dotenv;
extern crate rocket;
extern crate rocket_contrib;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_codegen;

use rocket_contrib::Template;


mod controller;
mod models;
mod forms;
mod database;
mod schema;


fn main() {
    rocket::ignite()
        .mount("/", routes![controller::pages::home])
        .mount("/recipes", routes![controller::recipes::create])
        .mount("/recipes", routes![controller::recipes::index])
        .mount("/recipes", routes![controller::recipes::show])
        .mount("/recipes", routes![controller::recipes::new])
        .mount("/recipes", routes![controller::recipes::edit])
        .mount("/recipes", routes![controller::recipes::update])
        .mount("/recipes", routes![controller::recipes::delete])
        .attach(Template::fairing())
        .launch();
}
