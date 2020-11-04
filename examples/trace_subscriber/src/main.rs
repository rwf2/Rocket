use rocket::trace;

#[rocket::get("/hello/<name>/<age>")]
fn hello(name: String, age: u8) -> String {
    trace::info!("saying hello...");
    greet(name, Some(age))
}

#[rocket::get("/hello/<name>")]
fn hi(name: String) -> String {
    trace::info!(?name, "saying hi to");
    greet(name, None)
}

#[trace::instrument(level = "info")]
fn greet(name: String, age: Option<u8>) -> String {
    trace::debug!(?age);
    if let Some(age) = age {
        trace::debug!("found an age, saying hello");
        format!("Hello, {} year old named {}!", age, name)
    } else {
        trace::debug!("no age, just saying hi...");
        name
    }
}

#[rocket::launch]
fn rocket() -> rocket::Rocket {
    use trace::prelude::*;

    let figment = rocket::Config::figment();
    let config = rocket::Config::from(&figment);

    // Use Rocket's default log filter...
    let filter = rocket::trace::filter_layer(config.log_level)
        // ...but enable the debug level for the app crate.
        .add_directive("trace_subscriber=debug".parse().unwrap());

    rocket::trace::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(filter)
        .init();

    rocket::custom(figment).mount("/", rocket::routes![hello, hi])
}
