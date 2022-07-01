use rocket::{Rocket, Build, Config};
use rocket::fairing::{self, Fairing, Info, Kind};
use rocket::error::ErrorKind;

struct Singleton(Kind, Kind, bool);

#[rocket::async_trait]
impl Fairing for Singleton {
    fn info(&self) -> Info {
        Info {
            name: "Singleton",
            kind: self.0
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> fairing::Result {
        if self.2 {
            Ok(rocket.attach(Singleton(self.1, self.1, false)))
        } else {
            Ok(rocket)
        }
    }
}


// Have => two `Singleton`s. This is okay; we keep the latter.
#[rocket::async_test]
async fn recursive_singleton_ok() {
    let result = rocket::custom(Config::debug_default())
        .attach(Singleton(Kind::Ignite | Kind::Singleton, Kind::Singleton, false))
        .attach(Singleton(Kind::Ignite | Kind::Singleton, Kind::Singleton, false))
        .ignite()
        .await;

    assert!(result.is_ok(), "{:?}", result);

    let result = rocket::custom(Config::debug_default())
        .attach(Singleton(Kind::Ignite | Kind::Singleton, Kind::Singleton, false))
        .attach(Singleton(Kind::Ignite | Kind::Singleton, Kind::Singleton, false))
        .attach(Singleton(Kind::Ignite | Kind::Singleton, Kind::Singleton, false))
        .attach(Singleton(Kind::Ignite | Kind::Singleton, Kind::Singleton, false))
        .ignite()
        .await;

    assert!(result.is_ok(), "{:?}", result);
}

// Have a `Singleton` add itself `on_ignite()`. Since it already ran, the one it
// adds can't be unique, so ensure we error in this case.
#[rocket::async_test]
async fn recursive_singleton_bad() {
    #[track_caller]
    fn assert_err(error: rocket::Error) {
        if let ErrorKind::FailedFairings(v) = error.kind() {
            assert_eq!(v.len(), 1);
            assert_eq!(v[0].name, "Singleton");
        } else {
            panic!("unexpected error: {:?}", error);
        }
    }

    let result = rocket::custom(Config::debug_default())
        .attach(Singleton(Kind::Ignite | Kind::Singleton, Kind::Ignite | Kind::Singleton, true))
        .ignite()
        .await;

    assert_err(result.unwrap_err());

    let result = rocket::custom(Config::debug_default())
        .attach(Singleton(Kind::Ignite | Kind::Singleton, Kind::Singleton, true))
        .ignite()
        .await;

    assert_err(result.unwrap_err());

    let result = rocket::custom(Config::debug_default())
        .attach(Singleton(Kind::Ignite, Kind::Singleton, true))
        .ignite()
        .await;

    assert_err(result.unwrap_err());
}
