use rocket::{Rocket, State, Build};
use rocket::fairing::AdHoc;
use rocket::tokio::sync::Barrier;

#[get("/barrier")]
async fn rendezvous(barrier: &State<Barrier>) -> &'static str {
    println!("Waiting for second task...");
    barrier.wait().await;
    "Rendezvous reached."
}

pub fn rocket() -> Rocket<Build> {
    rocket::build()
        .mount("/", routes![rendezvous])
        .attach(AdHoc::on_ignite("Add Channel", |rocket| async {
            rocket.manage(Barrier::new(2))
        }))
}

#[cfg(test)]
mod test {
    use super::rocket;
    use rocket::http::Status;

    #[rocket::async_test]
    async fn test_rendezvous() {
        use rocket::local::asynchronous::Client;

        let client = Client::tracked(rocket()).await.unwrap();
        let req = client.get("/barrier");

        let (r1, r2) = rocket::tokio::join!(req.clone().dispatch(), req.dispatch());
        assert_eq!(r1.status(), r2.status());
        assert_eq!(r1.status(), Status::Ok);

        let (s1, s2) = (r1.into_string().await, r2.into_string().await);
        assert_eq!(s1, s2);
        assert_eq!(s1.unwrap(), "Rendezvous reached.");
    }
}
