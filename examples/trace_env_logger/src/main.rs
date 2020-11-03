use rocket::trace;

#[rocket::get("/hello/<name>/<age>")]
fn hello(name: String, age: u8) -> String {
    trace::info!(?name, age, "saying hello to");
    format!("Hello, {} year old named {}!", age, name)
}

#[rocket::get("/hello/<name>")]
fn hi(name: String) -> String {
    trace::info!(?name, "saying hi to");
    name
}

#[rocket::launch]
fn rocket() -> rocket::Rocket {
    env_logger::init();
    rocket::ignite().mount("/", rocket::routes![hello, hi])
}
