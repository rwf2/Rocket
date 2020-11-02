#[cfg(all(feature = "diesel_sqlite_pool", feature = "diesel_postgres_pool"))]
mod databases_tests {
    use rocket_contrib::databases::{database, diesel};

    #[database("foo")]
    struct TempStorage(diesel::SqliteConnection);

    #[database("bar")]
    struct PrimaryDb(diesel::PgConnection);
}

#[cfg(all(feature = "databases", feature = "sqlite_pool"))]
#[cfg(test)]
mod rusqlite_integration_test {
    use rocket_contrib::database;
    use rocket_contrib::databases::rusqlite;

    use rusqlite::types::ToSql;

    #[database("test_db")]
    struct SqliteDb(pub rusqlite::Connection);

    // Test to ensure that multiple databases of the same type can be used
    #[database("test_db_2")]
    struct SqliteDb2(pub rusqlite::Connection);

    #[rocket::async_test]
    async fn test_db() {
        use rocket::figment::{Figment, util::map};

        let options = map!["url" => ":memory:"];
        let config = Figment::from(rocket::Config::default())
            .merge(("databases", map!["test_db" => &options]))
            .merge(("databases", map!["test_db_2" => &options]));

        let rocket = rocket::custom(config)
            .attach(SqliteDb::fairing())
            .attach(SqliteDb2::fairing());

        let conn = SqliteDb::get_one(&rocket).await
            .expect("unable to get connection");

        // Rusqlite's `transaction()` method takes `&mut self`; this tests that
        // the &mut method can be called inside the closure passed to `run()`.
        conn.run(|conn| {
            let tx = conn.transaction().unwrap();
            let _: i32 = tx.query_row(
                "SELECT 1", &[] as &[&dyn ToSql], |row| row.get(0)
            ).expect("get row");

            tx.commit().expect("committed transaction");
        }).await;
    }
}
