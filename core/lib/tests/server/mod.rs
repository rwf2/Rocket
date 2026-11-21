//! Launches Rocket on a real port for tests that connect over the network.

use std::net::{Ipv4Addr, SocketAddr, TcpListener, UdpSocket};

use rocket::{Build, Rocket, Shutdown};
use rocket::fairing::AdHoc;
use rocket::tokio::sync::{oneshot, Mutex, MutexGuard};
use rocket::tokio::task::JoinHandle;

static BINDING: Mutex<()> = Mutex::const_new(());

/// Returns a guard to hold while binding a socket so that a port found by
/// `free_port()` is still free when Rocket binds to it.
#[allow(dead_code)]
pub async fn binding() -> MutexGuard<'static, ()> {
    BINDING.lock().await
}

/// Returns a port that's free for TCP and UDP: Rocket serves HTTP/3 on the UDP
/// port with the same number as its TCP port.
fn free_port() -> u16 {
    loop {
        let udp = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = udp.local_addr().unwrap().port();
        if TcpListener::bind((Ipv4Addr::LOCALHOST, port)).is_ok() {
            return port;
        }
    }
}

pub struct Server {
    pub addr: SocketAddr,
    pub shutdown: Shutdown,
    pub task: JoinHandle<()>,
}

impl Server {
    /// Launches `rocket` on a free port and returns once it has lifted off.
    pub async fn launch(rocket: Rocket<Build>) -> Server {
        let _binding = binding().await;
        let figment = rocket.figment().clone().merge(("port", free_port()));
        let (tx, rx) = oneshot::channel();
        let rocket = rocket.reconfigure(figment);
        let rocket = rocket.attach(AdHoc::on_liftoff("Send Address", |rocket| {
            Box::pin(async move {
                let addr = rocket.endpoints().find_map(|e| e.tcp()).unwrap();
                tx.send((addr, rocket.shutdown())).unwrap();
            })
        }));

        let task = rocket::tokio::spawn(async move { rocket.launch().await.unwrap(); });
        let (addr, shutdown) = rx.await.unwrap();
        Server { addr, shutdown, task }
    }

    pub async fn shutdown(self) {
        self.shutdown.notify();
        self.task.await.unwrap();
    }
}
