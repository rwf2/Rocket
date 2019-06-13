#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
#[macro_use] extern crate serde_derive;

#[cfg(test)] mod tests;

use std::io::{self, Read};

use rocket::{Request, response::content, data::Data};

#[derive(Debug, Serialize, Deserialize)]
struct Person {
    name: String,
    age: u8,
}

// In a `GET` request and all other non-payload supporting request types, the
// preferred media type in the Accept header is matched against the `format` in
// the route attribute. Note: if this was a real application, we'd use
// `rocket_contrib`'s built-in JSON support and return a `JsonValue` instead.
#[get("/<name>/<age>", format = "json")]
fn get_hello(name: String, age: u8) -> content::Json<String> {
    // In a real application, we'd use the JSON contrib type.
    let person = Person { name: name, age: age, };
    content::Json(serde_json::to_string(&person).unwrap())
}

// In a `POST` request and all other payload supporting request types, the
// content type is matched against the `format` in the route attribute.
//
// Note that `content::Json` simply sets the content-type to `application/json`.
// In a real application, we wouldn't use `serde_json` directly; instead, we'd
// use `contrib::Json` to automatically serialize a type into JSON.
#[post("/<age>", format = "plain", data = "<name_data>")]
fn post_hello(age: u8, name_data: Data) -> io::Result<content::Json<String>> {
    let mut name = String::with_capacity(32);
    name_data.open().take(32).read_to_string(&mut name)?;
    let person = Person { name: name, age: age, };
    Ok(content::Json(serde_json::to_string(&person).unwrap()))
}

#[catch(404)]
fn not_found(request: &Request<'_>) -> content::Html<String> {
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
        .register(catchers![not_found])
        .launch();
}
