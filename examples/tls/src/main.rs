use rocket::launch;

#[launch]
fn launch() -> _ {
    tls::rocket()
}
