extern crate rocket;
extern crate rocket_contrib;

#[cfg(all(feature = "diesel_sqlite_pool", feature = "diesel_postgres_pool"))]
mod databases_tests {
    use rocket_contrib::databases::{database, diesel};

    #[database("foo")]
    struct TempStorage(diesel::SqliteConnection);

    #[database("bar")]
    struct PrimaryDb(diesel::PgConnection);
}

#[cfg(test)]
mod db_integration_tests {
    use std::collections::BTreeMap;
    use rocket::config::{Config, Environment, Value};
    use rocket_contrib::databases::rusqlite::{self, Result, NO_PARAMS};
    use rocket_contrib::database;

    #[database("test_db")]
    struct SqliteDb(pub rusqlite::Connection);

    #[test]
    fn deref_mut_impl_present() {
        let mut test_db: BTreeMap<String, Value> = BTreeMap::new();
        let mut test_db_opts: BTreeMap<String, Value> = BTreeMap::new();
        test_db_opts.insert("url".into(), Value::String(":memory:".into()));
        test_db.insert("test_db".into(), Value::Table(test_db_opts));
        let config = Config::build(Environment::Development)
            .extra("databases", Value::Table(test_db))
            .finalize()
            .unwrap();

        let rocket = rocket::custom(config).attach(SqliteDb::fairing());
        let connection = SqliteDb::get_one(&rocket);

        assert!(connection.is_some());

        let result: Result<i32> = connection.unwrap().query_row("SELECT 1", NO_PARAMS, |row| row.get(0));

        println!("{:#?}", result);

        assert!(result.is_ok());
    }
}
