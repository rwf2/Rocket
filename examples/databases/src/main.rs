#[macro_use] extern crate rocket;
#[macro_use] extern crate rocket_sync_db_pools;

#[cfg(test)] mod tests;

mod sqlx;
mod diesel_sqlite;
mod rusqlite;

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(sqlx::stage())
        .attach(rusqlite::stage())
        .attach(diesel_sqlite::stage())
}
