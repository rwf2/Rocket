use rocket::config::Config;
use rocket::fairing::AdHoc;
use rocket::futures::channel::oneshot;

#[rocket::async_test]
async fn on_ignite_fairing_can_inspect_port() {
    let (tx, rx) = oneshot::channel();
    let rocket = rocket::custom(Config {
        port: 0,
        ..Config::debug_default()
    })
    .attach(AdHoc::on_liftoff("Send Port -> Channel", move |rocket| {
        Box::pin(async move {
            tx.send(rocket.config().port).unwrap();
        })
    }));

    rocket::tokio::spawn(rocket.launch());
    assert_ne!(rx.await.unwrap(), 0);
}
