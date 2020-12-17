#[macro_use]
extern crate rocket;

use rocket::config::Config;
use rocket::fairing::AdHoc;
use rocket::figment::Figment;
use rocket::futures::channel::oneshot;

#[rocket::async_test]
async fn on_launch_fairing_can_inspect_port() {
    let (tx, rx) = oneshot::channel();
    let mut config = Config::default();
    config.port = 0;
    tokio::spawn(
        rocket::custom(Figment::from(config))
            .mount("/", routes![])
            .attach(AdHoc::on_launch("Get assigned port", move |rocket| {
                tx.send(rocket.config().port).unwrap();
            }))
            .launch(),
    );
    assert!(rx.await.unwrap() != 0);
}
