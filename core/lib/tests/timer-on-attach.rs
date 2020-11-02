#[rocket::async_test]
async fn test_await_timer_inside_attach() {

    async fn do_async_setup() {
        // By using a timer or I/O resource, we ensure that do_async_setup will
        // deadlock if no thread is able to tick the time or I/O drivers.
        rocket::tokio::time::delay_for(std::time::Duration::from_millis(100)).await;
    }

    rocket::ignite()
        .attach(rocket::fairing::AdHoc::on_attach("1", |rocket| async {
            do_async_setup().await;
            Ok(rocket)
        }));
}
