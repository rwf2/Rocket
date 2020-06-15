#![feature(proc_macro_hygiene)]

#[macro_use] extern crate rocket;
#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_migrations;
#[macro_use] extern crate log;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate rocket_contrib;

mod task;
#[cfg(test)] mod tests;

use rocket::Rocket;
use rocket::fairing::AdHoc;
use rocket::request::{Form, FlashMessage};
use rocket::response::{Flash, Redirect};
use rocket_contrib::{templates::Template, serve::StaticFiles};
use diesel::SqliteConnection;

use crate::task::{Task, Todo};

// This macro from `diesel_migrations` defines an `embedded_migrations` module
// containing a function named `run`. This allows the example to be run and
// tested without any outside setup of the database.
embed_migrations!();

#[database("sqlite_database")]
pub struct DbConn(SqliteConnection);

#[derive(Debug, Serialize)]
struct Context{ msg: Option<(String, String)>, tasks: Vec<Task> }

impl Context {
    pub fn err(conn: &SqliteConnection, msg: String) -> Context {
        Context{msg: Some(("error".to_string(), msg)), tasks: Task::all(conn)}
    }

    pub fn raw(conn: &SqliteConnection, msg: Option<(String, String)>) -> Context {
        Context{msg: msg, tasks: Task::all(conn)}
    }
}

#[post("/", data = "<todo_form>")]
async fn new(todo_form: Form<Todo>, mut conn: DbConn) -> Flash<Redirect> {
    conn.run(|conn| {
        let todo = todo_form.into_inner();
        if todo.description.is_empty() {
            Flash::error(Redirect::to("/"), "Description cannot be empty.")
        } else if Task::insert(todo, &conn) {
            Flash::success(Redirect::to("/"), "Todo successfully added.")
        } else {
            Flash::error(Redirect::to("/"), "Whoops! The server failed.")
        }
    }).await
}

#[put("/<id>")]
async fn toggle(id: i32, mut conn: DbConn) -> Result<Redirect, Template> {
    conn.run(move |conn| {
        if Task::toggle_with_id(id, &conn) {
            Ok(Redirect::to("/"))
        } else {
            Err(Template::render("index", &Context::err(&conn, "Couldn't toggle task.".to_string())))
        }
    }).await
}

#[delete("/<id>")]
async fn delete(id: i32, mut conn: DbConn) -> Result<Flash<Redirect>, Template> {
    conn.run(move |conn| {
        if Task::delete_with_id(id, &conn) {
            Ok(Flash::success(Redirect::to("/"), "Todo was deleted."))
        } else {
            Err(Template::render("index", &Context::err(&conn, "Couldn't delete task.".to_string())))
        }
    }).await
}

#[get("/")]
async fn index(msg: Option<FlashMessage<'_, '_>>, mut conn: DbConn) -> Template {
    let msg = msg.map(|m| (m.name().to_string(), m.msg().to_string()));

    conn.run(|conn| {
        Template::render("index", Context::raw(&conn, msg))
    }).await
}

async fn run_db_migrations(mut rocket: Rocket) -> Result<Rocket, Rocket> {
    let mut conn = DbConn::get_one(rocket.inspect().await).expect("database connection");
    conn.run(move |c| {
        match embedded_migrations::run(&*c) {
            Ok(()) => Ok(rocket),
            Err(e) => {
                error!("Failed to run database migrations: {:?}", e);
                Err(rocket)
            }
        }
    }).await
}

#[rocket::launch]
fn rocket() -> Rocket {
    rocket::ignite()
        .attach(DbConn::fairing())
        .attach(AdHoc::on_attach("Database Migrations", run_db_migrations))
        .mount("/", StaticFiles::from("static/"))
        .mount("/", routes![index])
        .mount("/todo", routes![new, toggle, delete])
        .attach(Template::fairing())
}
