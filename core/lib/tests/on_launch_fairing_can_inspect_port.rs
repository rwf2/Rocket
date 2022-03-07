use rocket::config::Config;
use rocket::fairing::AdHoc;
use rocket::futures::channel::oneshot;
use rocket::config::BindableAddr;
use std::net::{Ipv4Addr, SocketAddr};

#[rocket::async_test]
async fn on_ignite_fairing_can_inspect_port() {
    let (tx, rx) = oneshot::channel();
    let rocket = rocket::custom(
        Config {
            address: BindableAddr::Tcp(
                SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), 0)
            ),
            ..Config::debug_default()
        }
    ).attach(AdHoc::on_liftoff("Send Port -> Channel", move |rocket| {
        Box::pin(async move {
            tx.send(rocket.config().address.port()).unwrap();
        })
    }));

    rocket::tokio::spawn(rocket.launch());
    assert_eq!(rx.await.unwrap(), Some(0));
}
