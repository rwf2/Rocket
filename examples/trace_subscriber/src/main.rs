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
    use trace::prelude::*;

    let figment = rocket::Config::figment();
    let config = rocket::Config::from(&figment);

    rocket::trace::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(rocket::trace::filter_layer(config.log_level))
        .init();

    rocket::custom(figment).mount("/", rocket::routes![hello, hi])
}
