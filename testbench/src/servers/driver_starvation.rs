//! Regression for #2239: a blocked worker must not starve the runtime's I/O driver.

use std::time::Duration;

use ipc_channel::ipc::{self, IpcOneShotServer, IpcSender};

use crate::prelude::*;

#[get("/")]
fn index() -> &'static str {
    "still running"
}

pub fn test_driver_starvation() -> Result<()> {
    let (ready, name) = IpcOneShotServer::<IpcSender<()>>::new()?;
    let server = spawn!(name: String => {
        std::env::set_var("ROCKET_WORKERS", "2");
        Rocket::default()
            .reconfigure_with_toml("[default]\nwake_interval = 50")
            .mount("/", routes![index])
            .attach(AdHoc::on_liftoff("Block Worker", move |_| Box::pin(async move {
                tokio::spawn(async move {
                    // Let workers park, then block the worker that polls the timer.
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    let (release, wait) = ipc::channel::<()>().unwrap();
                    IpcSender::connect(name).unwrap().send(release).unwrap();
                    let _ = wait.recv();
                });
            })))
    })?;

    // Dropping `release` unblocks the worker, including on request failure.
    let (_, release) = ready.accept().unwrap();
    let client = Client::default();
    let response = client.get(&server, "/")?.timeout(Duration::from_secs(1)).send()?;
    assert_eq!(response.status(), 200);
    assert_eq!(response.text()?, "still running");
    drop(release);
    Ok(())
}

register!(test_driver_starvation);
