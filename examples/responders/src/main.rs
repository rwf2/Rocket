#[macro_use] extern crate rocket;

#[cfg(test)] mod tests;

/****************** `Result`, `Option` `NameFile` Responder *******************/

use std::{io, env};

use rocket::Config;
use rocket::data::Capped;
use rocket::fs::{NamedFile, TempFile};
use rocket::tokio::fs;

// Upload your `big_file.dat` by POSTing it to /upload.
// try `curl --data-binary @file.txt http://127.0.0.1:8000/stream/file`
const FILENAME: &str = "big_file.dat";

// This is a *raw* file upload, _not_ a multipart upload!
#[post("/file", data = "<file>")]
async fn upload(mut file: Capped<TempFile<'_>>, config: &Config) -> io::Result<String> {
    file.persist_to(config.temp_dir.relative().join(FILENAME)).await?;
    Ok(format!("{} bytes at {}", file.n.written, file.path().unwrap().display()))
}

#[get("/file")]
async fn file(config: &Config) -> Option<NamedFile> {
    NamedFile::open(config.temp_dir.relative().join(FILENAME)).await.ok()
}

#[delete("/file")]
async fn delete(config: &Config) -> Option<()> {
    fs::remove_file(config.temp_dir.relative().join(FILENAME)).await.ok()
}

/***************************** `Stream` Responder *****************************/

use rocket::tokio::select;
use rocket::tokio::time::{self, Duration};
use rocket::futures::stream::{repeat, StreamExt};

use rocket::Shutdown;
use rocket::response::stream::TextStream;

#[get("/stream/hi")]
fn many_his() -> TextStream![&'static str] {
    TextStream(repeat("hi").take(100))
}

#[get("/stream/hi/<n>")]
fn one_hi_per_ms(mut shutdown: Shutdown, n: u8) -> TextStream![&'static str] {
    TextStream! {
        let mut interval = time::interval(Duration::from_millis(n.into()));
        loop {
            select! {
                _ = interval.tick() => yield "hi",
                _ = &mut shutdown => {
                    yield "goodbye";
                    break;
                }
            };
        }
    }
}

/***************************** `Redirect` Responder ***************************/

use rocket::response::Redirect;

#[get("/redir")]
fn redir_root() -> Redirect {
    Redirect::to(uri!(redir_login))
}

#[get("/redir/login")]
fn redir_login() -> &'static str {
    "Hi! Please log in before continuing."
}

#[get("/redir/<name>")]
fn maybe_redir(name: &str) -> Result<&'static str, Redirect> {
    match name {
        "Sergio" => Ok("Hello, Sergio!"),
        _ => Err(Redirect::to(uri!(redir_login))),
    }
}

/***************************** `content` Responders ***************************/

use rocket::Request;
use rocket::response::content;

// NOTE: This example explicitly uses the `RawJson` type from
// `response::content` for demonstration purposes. In a real application,
// _always_ prefer to use `rocket::serde::json::Json` instead!

// In a `GET` request and all other non-payload supporting request types, the
// preferred media type in the Accept header is matched against the `format` in
// the route attribute. Because the client can use non-specific media types like
// `*/*` in `Accept`, these first two routes would collide without `rank`.
#[get("/content", format = "xml", rank = 1)]
fn xml() -> content::RawXml<&'static str> {
    content::RawXml("<payload>I'm here</payload>")
}

#[get("/content", format = "json", rank = 2)]
fn json() -> content::RawJson<&'static str> {
    content::RawJson(r#"{ "payload": "I'm here" }"#)
}

#[catch(404)]
fn not_found(request: &Request<'_>) -> content::RawHtml<String> {
    let html = match request.format() {
        Some(ref mt) if !(mt.is_xml() || mt.is_html()) => {
            format!("<p>'{}' requests are not supported.</p>", mt)
        }
        _ => format!("<p>Sorry, '{}' is an invalid path! Try \
                 /hello/&lt;name&gt;/&lt;age&gt; instead.</p>",
                 request.uri())
    };

    content::RawHtml(html)
}

/******************************* `Either` Responder ***************************/

use rocket::Either;
use rocket::response::content::{RawJson, RawMsgPack};
use rocket::http::uncased::AsUncased;

// NOTE: In a real application, we'd use `Json` and `MsgPack` from
// `rocket::serde`, which perform automatic serialization of responses and
// automatically set the `Content-Type`.
#[get("/content/<kind>")]
fn json_or_msgpack(kind: &str) -> Either<RawJson<&'static str>, RawMsgPack<&'static [u8]>> {
    if kind.as_uncased() == "msgpack" {
        Either::Right(RawMsgPack(&[162, 104, 105]))
    } else {
        Either::Left(RawJson("\"hi\""))
    }
}

/******************************* Custom Responder *****************************/

use std::borrow::Cow;

use rocket::response::content::RawHtml;

#[derive(Responder)]
enum StoredData {
    File(Option<NamedFile>),
    String(Cow<'static, str>),
    Bytes(Vec<u8>),
    #[response(status = 401)]
    NotAuthorized(RawHtml<&'static str>),
}

#[derive(FromFormField, UriDisplayQuery)]
enum Kind {
    File,
    String,
    Bytes
}

#[get("/custom?<kind>")]
async fn custom(kind: Option<Kind>) -> StoredData {
    match kind {
        Some(Kind::File) => {
            let path = env::temp_dir().join(FILENAME);
            StoredData::File(NamedFile::open(path).await.ok())
        },
        Some(Kind::String) => StoredData::String("Hey, I'm some data.".into()),
        Some(Kind::Bytes) => StoredData::Bytes(vec![72, 105]),
        None => StoredData::NotAuthorized(RawHtml("No no no!"))
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![many_his, one_hi_per_ms, file, upload, delete])
        .mount("/", routes![redir_root, redir_login, maybe_redir])
        .mount("/", routes![xml, json, json_or_msgpack])
        .mount("/", routes![custom])
        .register("/", catchers![not_found])
}
