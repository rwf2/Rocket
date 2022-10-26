#[cfg(all(feature = "diesel_sqlite_pool"))]
#[cfg(test)]
mod sqlite_shutdown_test {
    use rocket::{async_run, Build, Rocket};
    use rocket_sync_db_pools::database;

    #[database("test")]
    struct Pool(diesel::SqliteConnection);

    async fn rocket() -> Rocket<Build> {
        use rocket::figment::{util::map, Figment};

        let options = map!["url" => ":memory:"];
        let config = Figment::from(rocket::Config::debug_default())
            .merge(("databases", map!["test" => &options]));

        rocket::custom(config).attach(Pool::fairing())
    }

    #[test]
    fn test_shutdown() {
        let _ = async_run(
            async {
                let rocket = rocket().await.ignite().await.expect("unable to ignite");
                // request shutdown
                rocket.shutdown().notify();
                rocket.launch().await.expect("unable to launch")
            },
            1,
            32,
            true, // if `force_end` is set to true, then the runtime is stopped before the Pool is dropped
            "rocket-worker-shutdown-test-thread",
        );
    }
}
