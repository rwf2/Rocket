#[macro_use] extern crate rocket;

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use rocket::fairing::AdHoc;

// Want to test:
//
//   * stalled connection + sleep in shutdown -> conn closed
//   * stalled shutdown fairing stalls shutdown but not > grace + mercy
//     - sleep < grace + mercy
//     - sleep > grace + mercy

#[derive(Default)]
struct Flags {
    liftoff: AtomicBool,
    shutdown: AtomicUsize
}

#[test]
fn shutdown_fairing_runs() {
    use rocket::local::blocking::Client;

    let rocket = rocket::build()
        .manage(Flags::default())
        .attach(AdHoc::on_liftoff("Liftoff Flag", |rocket| Box::pin(async move {
            let flags = rocket.state::<Flags>().unwrap();
            flags.liftoff.store(true, Ordering::SeqCst);
        })))
        .attach(AdHoc::on_shutdown("Shutdown Flag", |rocket| Box::pin(async move {
            let flags = rocket.state::<Flags>().unwrap();
            flags.shutdown.fetch_add(1, Ordering::SeqCst);
        })));

    let client = Client::debug(rocket).unwrap();
    let flags = client.rocket().state::<Flags>().unwrap();
    assert!(flags.liftoff.load(Ordering::SeqCst));
    assert_eq!(0, flags.shutdown.load(Ordering::SeqCst));

    let rocket = client.terminate();
    let flags = rocket.state::<Flags>().unwrap();
    assert_eq!(1, flags.shutdown.load(Ordering::SeqCst));
}

#[async_test]
async fn async_shutdown_fairing_runs() {
    use rocket::local::asynchronous::Client;

    let rocket = rocket::build()
        .manage(Flags::default())
        .attach(AdHoc::on_liftoff("Liftoff Flag", |rocket| Box::pin(async move {
            let flags = rocket.state::<Flags>().unwrap();
            flags.liftoff.store(true, Ordering::SeqCst);
        })))
        .attach(AdHoc::on_shutdown("Shutdown Flag", |rocket| Box::pin(async move {
            let flags = rocket.state::<Flags>().unwrap();
            flags.shutdown.fetch_add(1, Ordering::SeqCst);
        })));

    let client = Client::debug(rocket).await.unwrap();
    let flags = client.rocket().state::<Flags>().unwrap();
    assert!(flags.liftoff.load(Ordering::SeqCst));
    assert_eq!(0, flags.shutdown.load(Ordering::SeqCst));

    let rocket = client.terminate().await;
    let flags = rocket.state::<Flags>().unwrap();
    assert_eq!(1, flags.shutdown.load(Ordering::SeqCst));
}

#[async_test]
async fn multiple_shutdown_fairing_runs() {
    use rocket::local::asynchronous::Client;

    let rocket = rocket::build()
        .manage(Flags::default())
        .attach(AdHoc::on_shutdown("Shutdown Flag 1", |rocket| Box::pin(async move {
            let flags = rocket.state::<Flags>().unwrap();
            flags.shutdown.fetch_add(1, Ordering::SeqCst);
        })))
        .attach(AdHoc::on_shutdown("Shutdown Flag 2", |rocket| Box::pin(async move {
            let flags = rocket.state::<Flags>().unwrap();
            flags.shutdown.fetch_add(1, Ordering::SeqCst);
        })));

    let client = Client::debug(rocket).await.unwrap();
    let flags = client.rocket().state::<Flags>().unwrap();
    assert_eq!(0, flags.shutdown.load(Ordering::SeqCst));

    let rocket = client.terminate().await;
    let flags = rocket.state::<Flags>().unwrap();
    assert_eq!(2, flags.shutdown.load(Ordering::SeqCst));
}

#[async_test]
async fn async_slow_shutdown_doesnt_elongate_grace() {
    use rocket::local::asynchronous::Client;

    let mut config = rocket::Config::debug_default();
    config.shutdown.grace = 1;
    config.shutdown.mercy = 1;

    let rocket = rocket::build()
        .manage(Flags::default())
        .configure(config)
        .attach(AdHoc::on_shutdown("Slow Shutdown", |rocket| Box::pin(async move {
            tokio::time::sleep(std::time::Duration::from_secs(4)).await;
            let flags = rocket.state::<Flags>().unwrap();
            flags.shutdown.fetch_add(1, Ordering::SeqCst);
        })));

    let client = Client::debug(rocket).await.unwrap();
    let flags = client.rocket().state::<Flags>().unwrap();
    assert_eq!(0, flags.shutdown.load(Ordering::SeqCst));

    let start = std::time::Instant::now();
    let rocket = client.terminate().await;
    let elapsed = start.elapsed();

    let flags = rocket.state::<Flags>().unwrap();
    assert!(elapsed > std::time::Duration::from_secs(2));
    assert!(elapsed < std::time::Duration::from_secs(5));
    assert_eq!(1, flags.shutdown.load(Ordering::SeqCst));
}

#[test]
fn background_tasks_dont_prevent_terminate() {
    use rocket::local::blocking::Client;

    #[get("/")]
    fn index() {
        tokio::task::spawn(async {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        });

        tokio::task::spawn_blocking(|| {
            std::thread::sleep(std::time::Duration::from_secs(10));
        });
    }

    let mut config = rocket::Config::debug_default();
    config.shutdown.grace = 1;
    config.shutdown.mercy = 1;

    let rocket = rocket::build().configure(config).mount("/", routes![index]);

    let client = Client::debug(rocket).unwrap();
    let response = client.get("/").dispatch();
    assert!(response.status().class().is_success());
    drop(response);
    let _ = client.terminate();
}
