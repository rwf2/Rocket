#![cfg(feature = "sqlx_sqlite")]

use std::time::Duration;

use rocket::error::ErrorKind::FailedFairings;
use rocket::fairing::Fairing;
use rocket::figment::Figment;
use rocket_db_pools::{sqlx, Config, Database, Error};

#[derive(Database)]
#[database("sqlite")]
struct Db(sqlx::SqlitePool);

fn config() -> Figment {
    Figment::from(rocket::Config::debug_default()).merge(("databases.sqlite", Config {
        url: "sqlite::memory:".into(),
        min_connections: Some(0),
        max_connections: 1,
        connect_timeout: 1,
        idle_timeout: None,
    }))
}

#[rocket::async_test]
async fn busy_timeout() {
    let rocket = rocket::custom(config()).attach(Db::init()).ignite().await.unwrap();
    let db = Db::fetch(&rocket).unwrap();
    let mut conn = db.acquire().await.unwrap();
    let timeout: i64 = sqlx::query_scalar("PRAGMA busy_timeout")
        .fetch_one(&mut *conn).await.unwrap();
    assert_eq!(timeout, 1000);
    drop(conn);
    db.close().await;
}

#[rocket::async_test]
async fn stale_extensions_key_does_not_block_ignition() {
    let figment = config()
        .merge(("databases.sqlite.extensions", ["rocket_nonexistent_sqlite_extension"]));
    let rocket = rocket::custom(figment).attach(Db::init()).ignite().await.unwrap();
    Db::fetch(&rocket).unwrap().close().await;
}

#[rocket::async_test]
async fn native_options_override_defaults() {
    let rocket = rocket::custom(config()).attach(Db::custom(|_, (pool, connection)| {
        let pool = pool.after_connect(|connection, _| Box::pin(async move {
            sqlx::query("PRAGMA cache_size = -37").execute(connection).await?;
            Ok(())
        }));

        (pool, connection.busy_timeout(Duration::from_millis(73)))
    })).ignite().await.unwrap();

    let db = Db::fetch(&rocket).unwrap();
    let mut connection = db.acquire().await.unwrap();
    let timeout: i64 = sqlx::query_scalar("PRAGMA busy_timeout")
        .fetch_one(&mut *connection).await.unwrap();
    assert_eq!(timeout, 73);
    let cache: i64 = sqlx::query_scalar("PRAGMA cache_size")
        .fetch_one(&mut *connection).await.unwrap();
    assert_eq!(cache, -37);
    drop(connection);
    db.close().await;
}

#[rocket::async_test]
async fn callback_error_aborts_ignition() {
    let error = rocket::custom(config()).attach(Db::try_custom(|_, _| {
        Err(Error::Init(sqlx::Error::Configuration("callback failed".into())))
    })).ignite().await.unwrap_err();

    match error.kind() {
        FailedFairings(failures) => assert_eq!(failures[0].name, Db::init().info().name),
        _ => panic!("Wrong kind of launch error"),
    }
}
