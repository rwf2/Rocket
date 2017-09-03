#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

#[cfg(test)] mod tests;

use rocket::Request;
use rocket::response::content;

#[derive(Debug, Serialize, Deserialize)]
struct Person {
    name: String,
    age: u8,
}

// In a `GET` request and all other non-payload supporting request types, the
// preferred media type in the Accept header is matched against the `format` in
// the route attribute.
#[get("/<name>/<age>", format = "application/json")]
fn get_hello(name: String, age: u8) -> content::Json<String> {
    // In a real application, we'd use the JSON contrib type.
    let person = Person { name: name, age: age, };
    content::Json(serde_json::to_string(&person).unwrap())
}

// In a `POST` request and all other payload supporting request types, the
// content type is matched against the `format` in the route attribute.
#[post("/<age>", format = "text/plain", data = "<name>")]
fn post_hello(age: u8, name: String) -> content::Json<String> {
    let person = Person { name: name, age: age, };
    content::Json(serde_json::to_string(&person).unwrap())
}

#[error(404)]
fn not_found(request: &Request) -> content::Html<String> {
    let html = match request.format() {
        Some(ref mt) if !mt.is_json() && !mt.is_plain() => {
            format!("<p>'{}' requests are not supported.</p>", mt)
        }
        _ => format!("<p>Sorry, '{}' is an invalid path! Try \
                 /hello/&lt;name&gt;/&lt;age&gt; instead.</p>",
                 request.uri())
    };

    content::Html(html)
}

fn main() {
    rocket::ignite()
        .mount("/hello", routes![get_hello, post_hello])
        .catch(errors![not_found])
        .launch();
}
