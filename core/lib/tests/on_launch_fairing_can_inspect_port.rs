extern crate rocket_community as rocket;

use std::net::{Ipv4Addr, SocketAddr};

use rocket::config::Config;
use rocket::fairing::AdHoc;
use rocket::futures::channel::oneshot;
use rocket::listener::tcp::TcpListener;

#[rocket::async_test]
async fn on_ignite_fairing_can_inspect_port() {
    let (tx, rx) = oneshot::channel();
    let rocket = rocket::custom(Config::debug_default()).attach(AdHoc::on_liftoff(
        "Send Port -> Channel",
        move |rocket| {
            Box::pin(async move {
                let tcp = rocket.endpoints().find_map(|v| v.tcp());
                tx.send(tcp.unwrap().port()).expect("send okay");
            })
        },
    ));

    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    rocket::tokio::spawn(rocket.try_launch_on(TcpListener::bind(addr)));
    assert_ne!(rx.await.unwrap(), 0);
}
