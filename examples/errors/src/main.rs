#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

#[cfg(test)]
mod tests;

#[get("/hello/<name>/<age>")]
fn hello(name: &str, age: i8) -> String {
    format!("Hello, {} year old named {}!", age, name)
}

#[error(404)]
fn not_found(req: &rocket::Request) -> String {
    format!("<p>Sorry, but '{}' is not a valid path!</p>
            <p>Try visiting /hello/&lt;name&gt;/&lt;age&gt; instead.</p>",
            req.uri())
}

fn main() {
    rocket::ignite()
        .mount("/", routes![hello])
        .catch(errors![not_found])
        .launch();
}
