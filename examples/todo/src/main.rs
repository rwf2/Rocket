#[macro_use] extern crate rocket;
#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_migrations;
#[macro_use] extern crate log;
#[macro_use] extern crate rocket_contrib;

mod task;
#[cfg(test)] mod tests;

use rocket::Rocket;
use rocket::fairing::AdHoc;
use rocket::request::{Form, FlashMessage};
use rocket::response::{Flash, Redirect};
use rocket_contrib::{templates::Template, serve::{StaticFiles, crate_relative}};
use diesel::SqliteConnection;

use crate::task::{Task, Todo};

// This macro from `diesel_migrations` defines an `embedded_migrations` module
// containing a function named `run`. This allows the example to be run and
// tested without any outside setup of the database.
embed_migrations!();

#[database("sqlite_database")]
pub struct DbConn(SqliteConnection);

#[derive(Debug, serde::Serialize)]
struct Context {
    msg: Option<(String, String)>,
    tasks: Vec<Task>
}

impl Context {
    pub fn err(conn: &SqliteConnection, msg: String) -> Context {
        Context {
            msg: Some(("error".to_string(), msg)),
            tasks: Task::all(conn).unwrap_or_default()
        }
    }

    pub fn raw(conn: &SqliteConnection, msg: Option<(String, String)>) -> Context {
        match Task::all(conn) {
            Ok(tasks) => Context { msg, tasks },
            Err(e) => {
                error_!("DB Task::all() error: {}", e);
                Context {
                    msg: Some(("error".to_string(), "Couldn't access the task database.".to_string())),
                    tasks: vec![]
                }
            }
        }
    }
}

#[post("/", data = "<todo_form>")]
async fn new(todo_form: Form<Todo>, conn: DbConn) -> Flash<Redirect> {
    conn.run(|c| {
        let todo = todo_form.into_inner();
        if todo.description.is_empty() {
            Flash::error(Redirect::to("/"), "Description cannot be empty.")
        } else if let Err(e) = Task::insert(todo, c) {
            error_!("DB insertion error: {}", e);
            Flash::error(Redirect::to("/"), "Todo could not be inserted due an internal error.")
        } else {
            Flash::success(Redirect::to("/"), "Todo successfully added.")
        }
    }).await
}

#[put("/<id>")]
async fn toggle(id: i32, conn: DbConn) -> Result<Redirect, Template> {
    conn.run(move |c| {
        Task::toggle_with_id(id, c)
            .map(|_| Redirect::to("/"))
            .map_err(|e| {
                error_!("DB toggle({}) error: {}", id, e);
                Template::render("index", Context::err(c, "Failed to toggle task.".to_string()))
            })
    }).await
}

#[delete("/<id>")]
async fn delete(id: i32, conn: DbConn) -> Result<Flash<Redirect>, Template> {
    conn.run(move |c| {
        Task::delete_with_id(id, c)
            .map(|_| Flash::success(Redirect::to("/"), "Todo was deleted."))
            .map_err(|e| {
                error_!("DB deletion({}) error: {}", id, e);
                Template::render("index", Context::err(c, "Failed to delete task.".to_string()))
            })
    }).await
}

#[get("/")]
async fn index(msg: Option<FlashMessage<'_, '_>>, conn: DbConn) -> Template {
    let msg = msg.map(|m| (m.name().to_string(), m.msg().to_string()));
    conn.run(|c| {
        Template::render("index", match msg {
            Some(msg) => Context::raw(c, Some(msg)),
            None => Context::raw(c, None),
        })
    }).await
}

async fn run_db_migrations(mut rocket: Rocket) -> Result<Rocket, Rocket> {
    let conn = DbConn::get_one(rocket.inspect().await).await.expect("database connection");
    conn.run(|c| {
        match embedded_migrations::run(c) {
            Ok(()) => Ok(rocket),
            Err(e) => {
                error!("Failed to run database migrations: {:?}", e);
                Err(rocket)
            }
        }
    }).await
}

#[launch]
fn rocket() -> Rocket {
    rocket::ignite()
        .attach(DbConn::fairing())
        .attach(AdHoc::on_attach("Database Migrations", run_db_migrations))
        .mount("/", StaticFiles::from(crate_relative!("/static")))
        .mount("/", routes![index])
        .mount("/todo", routes![new, toggle, delete])
        .attach(Template::fairing())
}
