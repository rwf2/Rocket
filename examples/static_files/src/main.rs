#[macro_use] extern crate rocket;

#[cfg(test)] mod tests;

use rocket_contrib::serve::{StaticFiles, crate_relative};

// If we wanted or needed to serve files manually, we'd use `NamedFile`. Always
// prefer to use `StaticFiles`!
mod manual {
    use rocket::response::NamedFile;

    #[rocket::get("/rocket-icon.jpg")]
    pub async fn icon() -> Option<NamedFile> {
        NamedFile::open("static/rocket-icon.jpg").await.ok()
    }
}

#[launch]
fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/", routes![manual::icon])
        .mount("/", StaticFiles::from(crate_relative!("/static")))
}
