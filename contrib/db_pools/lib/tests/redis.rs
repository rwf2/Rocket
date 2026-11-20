#![cfg(feature = "deadpool_redis")]

use std::time::Duration;

use deadpool::managed::PoolError;
use rocket::figment::{Figment, providers::Serialized};
use rocket_db_pools::{deadpool_redis, Config, Error, Pool};

fn config() -> Figment {
    Figment::from(Serialized::defaults(Config {
        url: "redis://localhost".into(),
        ..Config::default()
    }))
}

#[rocket::async_test]
async fn invalid_response_timeouts_are_config_errors() {
    let figment = config().merge(("response_timeout", -1));
    let result = <deadpool_redis::Pool as Pool>::init(&figment).await;
    assert!(matches!(result, Err(Error::Config(_))));
}

#[rocket::async_test]
async fn connection_creation_uses_pool_timeout() {
    use rocket::tokio::{net::TcpListener, time::timeout};

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let figment = config()
        .merge(("url", format!("redis://{}", listener.local_addr().unwrap())))
        .merge(("connect_timeout", 2));
    let pool = <deadpool_redis::Pool as Pool>::init(&figment).await.unwrap();

    // Accept the connection without responding to Redis's setup commands.
    let (accepted, result) = timeout(Duration::from_secs(5), async {
        rocket::futures::future::join(listener.accept(), Pool::get(&pool)).await
    }).await.unwrap();
    let _connection = accepted.unwrap();
    assert!(matches!(result, Err(Error::Get(PoolError::Timeout(
        deadpool::managed::TimeoutType::Create
    )))));
    pool.close();
}
