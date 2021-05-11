#[macro_use] extern crate rocket;

mod paste_id;
#[cfg(test)] mod tests;

use std::io;

use rocket::State;
use rocket::data::{Data, ToByteUnit};
use rocket::http::uri::Absolute;
use rocket::response::content::Plain;
use rocket::tokio::fs::{self, File};

use crate::paste_id::PasteId;

const HOST: &str = "http://localhost:8000";
const ID_LENGTH: usize = 3;

#[post("/", data = "<paste>")]
async fn upload(paste: Data, host: &State<Absolute<'_>>) -> io::Result<String> {
    let id = PasteId::new(ID_LENGTH);
    paste.open(128.kibibytes()).into_file(id.file_path()).await?;

    // TODO: Ok(uri!(HOST, retrieve: id))
    let host = host.inner().clone();
    Ok(host.with_origin(uri!(retrieve: id)).to_string())
}

#[get("/<id>")]
async fn retrieve(id: PasteId<'_>) -> Option<Plain<File>> {
    File::open(id.file_path()).await.map(Plain).ok()
}

#[delete("/<id>")]
async fn delete(id: PasteId<'_>) -> Option<()> {
    fs::remove_file(id.file_path()).await.ok()
}

#[get("/")]
fn index() -> &'static str {
    "
    USAGE

      POST /

          accepts raw data in the body of the request and responds with a URL of
          a page containing the body's content

          EXAMPLE: curl --data-binary @file.txt http://localhost:8000

      GET /<id>

          retrieves the content for the paste with id `<id>`
    "
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .manage(Absolute::parse(HOST).expect("valid host"))
        .mount("/", routes![index, upload, delete, retrieve])
}
