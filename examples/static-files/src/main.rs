#[cfg(test)] mod tests;

use rocket::fs::{FileServer, relative};

// If we wanted or needed to serve files manually, we'd use `NamedFile`. Always
// prefer to use `FileServer`!
mod manual {
    use std::path::{PathBuf, Path};
    use rocket::fs::NamedFile;

    #[rocket::get("/second/<path..>")]
    pub async fn second(path: PathBuf) -> Option<NamedFile> {
        let mut path = Path::new(super::relative!("static")).join(path);
        if path.is_dir() {
            path.push("index.html");
        }

        NamedFile::open(path).await.ok()
    }
}

#[rocket::launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", rocket::routes![manual::second])
        .mount("/", FileServer::from(relative!("static")))
}
