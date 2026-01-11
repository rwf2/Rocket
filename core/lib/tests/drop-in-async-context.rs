#[macro_use]
extern crate rocket;

use rocket::{Build, Config, Rocket};
use rocket::fairing::AdHoc;
use rocket::figment::Figment;

struct AsyncDropInAsync;

impl Drop for AsyncDropInAsync {
    fn drop(&mut self) {
        // Ensure that managed state is dropped inside of an async context by
        // ensuring that we do not panic when fetching the current runtime.
        //
        // Crates like rocket_sync_db_pools spawn tasks to asynchronously
        // complete pool shutdown which must be done in an async context or else
        // the spawn will panic. We want to ensure that does not happen.
        let _ = rocket::tokio::runtime::Handle::current();
    }
}

fn rocket() -> Rocket<Build> {
    let figment = Figment::from(Config::debug_default())
        .merge(("address", "tcp:127.0.0.1:0"));

    rocket::custom(figment)
        .manage(AsyncDropInAsync)
        .attach(AdHoc::on_liftoff("Shutdown", |rocket| Box::pin(async {
            rocket.shutdown().notify();
        })))
}

mod launch {
    #[launch]
    fn launch() -> _ {
        super::rocket()
    }

    #[test]
    fn test_launch() {
        main();
    }
}

mod main {
    #[rocket::main]
    async fn main() {
        super::rocket().launch().await.unwrap();
    }

    #[test]
    fn test_main() {
        main();
    }

    #[test]
    fn test_execute() {
        rocket::execute(async {
            super::rocket().launch().await.unwrap();
        });
    }

    #[test]
    fn test_execute_directly() {
        rocket::execute(super::rocket().launch()).unwrap();
    }
}
