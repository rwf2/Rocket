#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

#[cfg(test)] mod tests;

use rocket::Request;
use rocket::http::ContentType;
use rocket::response::content;

#[derive(Debug, Serialize, Deserialize)]
struct Person {
    name: String,
    age: i8,
}

// This shows how to manually serialize some JSON, but in a real application,
// we'd use the JSON contrib type.
#[get("/<name>/<age>", format = "application/json")]
fn hello(content_type: ContentType, name: String, age: i8) -> content::JSON<String> {
    let person = Person {
        name: name,
        age: age,
    };

    println!("ContentType: {}", content_type);
    content::JSON(serde_json::to_string(&person).unwrap())
}

#[error(404)]
fn not_found(request: &Request) -> content::HTML<String> {
    let html = match request.content_type() {
        Some(ref ct) if !ct.is_json() => {
            format!("<p>This server only supports JSON requests, not '{}'.</p>", ct)
        }
        _ => format!("<p>Sorry, '{}' is an invalid path! Try \
                 /hello/&lt;name&gt;/&lt;age&gt; instead.</p>",
                 request.uri())
    };

    content::HTML(html)
}

fn main() {
    rocket::ignite()
        .mount("/hello", routes![hello])
        .catch(errors![not_found])
        .launch();
}
