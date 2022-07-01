#[cfg(all(feature = "diesel_sqlite_pool", feature = "diesel_postgres_pool"))]
mod databases_tests {
    use rocket_sync_db_pools::{database, diesel};

    #[database("example")]
    struct ExampleDb(diesel::SqliteConnection);

    #[database("bar")]
    struct PrimaryDb(diesel::PgConnection);
}

#[cfg(all(feature = "databases", feature = "sqlite_pool"))]
#[cfg(test)]
mod rusqlite_integration_test {
    use rocket_sync_db_pools::{rusqlite, database};

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
        let config = Figment::from(rocket::Config::debug_default())
            .merge(("databases", map!["test_db" => &options]))
            .merge(("databases", map!["test_db_2" => &options]));

        let rocket = rocket::custom(config)
            .attach(SqliteDb::fairing())
            .attach(SqliteDb2::fairing())
            .ignite()
            .await
            .unwrap();

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

#[cfg(test)]
#[cfg(feature = "databases")]
mod sentinel_and_runtime_test {
    use rocket::{Rocket, Build};
    use r2d2::{ManageConnection, Pool};
    use rocket_sync_db_pools::{database, Poolable, PoolResult};
    use tokio::runtime::Runtime;

    struct ContainsRuntime(Runtime);
    struct TestConnection;

    impl ManageConnection for ContainsRuntime {
        type Connection = TestConnection;
        type Error = std::convert::Infallible;

        fn connect(&self) -> Result<Self::Connection, Self::Error> {
            Ok(TestConnection)
        }

        fn is_valid(&self, _conn: &mut Self::Connection) -> Result<(), Self::Error> {
            Ok(())
        }

        fn has_broken(&self, _conn: &mut Self::Connection) -> bool {
            false
        }
    }

    impl Poolable for TestConnection {
        type Manager = ContainsRuntime;
        type Error = ();

        fn pool(_db_name: &str, _rocket: &Rocket<Build>) -> PoolResult<Self> {
            let manager = ContainsRuntime(tokio::runtime::Runtime::new().unwrap());
            Ok(Pool::builder().build(manager)?)
        }
    }

    #[database("test_db")]
    struct TestDb(TestConnection);

    #[rocket::async_test]
    async fn test_drop_runtime() {
        use rocket::figment::{Figment, util::map};

        let config = Figment::from(rocket::Config::debug_default())
            .merge(("databases", map!["test_db" => map!["url" => ""]]));

        let rocket = rocket::custom(config).attach(TestDb::fairing());
        drop(rocket);
    }

    #[test]
    fn test_sentinel() {
        use rocket::{*, local::blocking::Client, error::ErrorKind::SentinelAborts};

        #[get("/")]
        fn use_db(_db: TestDb) {}

        let err = Client::debug_with(routes![use_db]).unwrap_err();
        assert!(matches!(err.kind(), SentinelAborts(vec) if vec.len() == 1));
    }
}
