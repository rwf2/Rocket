use rocket::config::Config;
use rocket::fairing::AdHoc;
use rocket::futures::channel::oneshot;

#[rocket::async_test]
async fn on_launch_fairing_can_inspect_port() {
    let (tx, rx) = oneshot::channel();
    let rocket = rocket::custom(Config { port: 0, ..Default::default() })
        .attach(AdHoc::on_launch("Send Port -> Channel", move |rocket| {
            tx.send(rocket.config().port).unwrap();
        }));

    rocket::tokio::spawn(rocket.launch());
    assert_ne!(rx.await.unwrap(), 0);
}
