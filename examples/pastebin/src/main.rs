#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate rand;

mod paste_id;
#[cfg(test)] mod tests;

use std::io;
use std::fs::File;
use std::path::Path;

use rocket::Data;
use rocket::response::content;

use paste_id::PasteID;

const HOST: &'static str = "http://localhost:8000";
const ID_LENGTH: usize = 3;

#[post("/", data = "<paste>")]
fn upload(paste: Data) -> io::Result<String> {
    let id = PasteID::new(ID_LENGTH);
    let filename = format!("upload/{id}", id = id);
    let url = format!("{host}/{id}\n", host = HOST, id = id);

    paste.stream_to_file(Path::new(&filename))?;
    Ok(url)
}

#[get("/<id>")]
fn retrieve(id: PasteID) -> Option<content::Plain<File>> {
    let filename = format!("upload/{id}", id = id);
    File::open(&filename).map(|f| content::Plain(f)).ok()
}

#[get("/")]
fn index() -> &'static str {
    "
    USAGE

      POST /

          accepts raw data in the body of the request and responds with a URL of
          a page containing the body's content

          EXMAPLE: curl --data-binary @file.txt http://localhost:8000

      GET /<id>

          retrieves the content for the paste with id `<id>`
    "
}

fn rocket() -> rocket::Rocket {
    rocket::ignite().mount("/", routes![index, upload, retrieve])
}

fn main() {
    rocket().launch();
}
