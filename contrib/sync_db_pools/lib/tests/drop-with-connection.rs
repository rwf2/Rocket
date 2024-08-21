#![cfg(feature = "diesel_sqlite_pool")]

use rocket::figment::Figment;
use rocket_sync_db_pools::database;

#[database("example")]
struct ExampleDb(diesel::SqliteConnection);

#[test]
fn can_drop_connection_in_sync_context() {
    let conn = rocket::execute(async {
        let figment = Figment::from(rocket::Config::debug_default())
            .merge(("databases.example.url", ":memory:"));

        let rocket = rocket::custom(figment)
            .attach(ExampleDb::fairing())
            .ignite().await
            .expect("rocket");

        ExampleDb::get_one(&rocket).await
            .expect("attach => connection")
    });

    drop(conn);
}
