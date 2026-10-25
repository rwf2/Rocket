#![cfg(feature = "sqlx_sqlite")]

use rocket::figment::{Figment, providers::Serialized};
use rocket_db_pools::{sqlx, Config, Error, Pool};

fn config() -> Figment {
    Figment::from(Serialized::defaults(Config {
        url: "sqlite::memory:".into(),
        min_connections: Some(1),
        max_connections: 1,
        connect_timeout: 1,
        idle_timeout: None,
        extensions: None,
    }))
}

#[rocket::async_test]
async fn options_and_shutdown() {
    use rocket::futures::FutureExt;

    let pool = <sqlx::SqlitePool as Pool>::init(&config()).await.unwrap();
    assert_eq!(pool.size(), 1);

    let mut conn = Pool::get(&pool).await.unwrap();
    let timeout: i64 = sqlx::query_scalar("PRAGMA busy_timeout")
        .fetch_one(&mut *conn).await.unwrap();
    assert_eq!(timeout, 1000);

    let result = Pool::get(&pool).await;
    assert!(matches!(result, Err(Error::Get(sqlx::Error::PoolTimedOut))));

    // Shutdown denies new connections while waiting for this one to return.
    let close = Pool::close(&pool);
    rocket::tokio::pin!(close);
    assert!(close.as_mut().now_or_never().is_none());
    assert!(matches!(Pool::get(&pool).await, Err(Error::Get(sqlx::Error::PoolClosed))));
    drop(conn);
    close.await;
}

#[rocket::async_test]
async fn initialization_connects_and_loads_extensions() {
    // Even with no minimum connections, initialization must connect eagerly.
    let figment = config()
        .merge(("min_connections", 0))
        .merge(("extensions", ["rocket_nonexistent_sqlite_extension"]));
    let result = <sqlx::SqlitePool as Pool>::init(&figment).await;
    assert!(matches!(result, Err(Error::Init(_))));

    let figment = config().merge(("url", "sqlite://?mode=invalid"));
    let result = <sqlx::SqlitePool as Pool>::init(&figment).await;
    assert!(matches!(result, Err(Error::Init(_))));
}
