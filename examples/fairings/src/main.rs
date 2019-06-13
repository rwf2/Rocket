#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

use std::io::Cursor;
use std::sync::atomic::{AtomicUsize, Ordering};

use rocket::{Request, State, Data, Response};
use rocket::fairing::{AdHoc, Fairing, Info, Kind};
use rocket::http::{Method, ContentType, Status};

struct Token(i64);

#[cfg(test)] mod tests;

#[derive(Default)]
struct Counter {
    get: AtomicUsize,
    post: AtomicUsize,
}

impl Fairing for Counter {
    fn info(&self) -> Info {
        Info {
            name: "GET/POST Counter",
            kind: Kind::Request | Kind::Response
        }
    }

    fn on_request(&self, request: &mut Request<'_>, _: &Data) {
        if request.method() == Method::Get {
            self.get.fetch_add(1, Ordering::Relaxed);
        } else if request.method() == Method::Post {
            self.post.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn on_response(&self, request: &Request<'_>, response: &mut Response<'_>) {
        if response.status() != Status::NotFound {
            return
        }

        if request.method() == Method::Get && request.uri().path() == "/counts" {
            let get_count = self.get.load(Ordering::Relaxed);
            let post_count = self.post.load(Ordering::Relaxed);

            let body = format!("Get: {}\nPost: {}", get_count, post_count);
            response.set_status(Status::Ok);
            response.set_header(ContentType::Plain);
            response.set_sized_body(Cursor::new(body));
        }
    }
}

#[put("/")]
fn hello() -> &'static str {
    "Hello, world!"
}

#[get("/token")]
fn token(token: State<'_, Token>) -> String {
    format!("{}", token.0)
}

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/", routes![hello, token])
        .attach(Counter::default())
        .attach(AdHoc::on_attach("Token State", |rocket| {
            println!("Adding token managed state...");
            let token_val = rocket.config().get_int("token").unwrap_or(-1);
            Ok(rocket.manage(Token(token_val)))
        }))
        .attach(AdHoc::on_launch("Launch Message", |_| {
            println!("Rocket is about to launch!");
        }))
        .attach(AdHoc::on_request("PUT Rewriter", |req, _| {
            println!("    => Incoming request: {}", req);
            if req.uri().path() == "/" {
                println!("    => Changing method to `PUT`.");
                req.set_method(Method::Put);
            }
        }))
        .attach(AdHoc::on_response("Response Rewriter", |req, res| {
            if req.uri().path() == "/" {
                println!("    => Rewriting response body.");
                res.set_sized_body(Cursor::new("Hello, fairings!"));
            }
        }))
}

fn main() {
    rocket().launch();
}
