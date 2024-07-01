//! File serving, file accepting, and file metadata types.

mod server;
mod named_file;
mod temp_file;
mod file_name;

pub mod rewrite;

pub use server::*;
pub use named_file::*;
pub use temp_file::*;
pub use file_name::*;

crate::export! {
    /// Generates a crate-relative version of a path.
    ///
    /// This macro is primarily intended for use with [`FileServer`] to serve
    /// files from a path relative to the crate root.
    ///
    /// The macro accepts one parameter, `$path`, an absolute or (preferably)
    /// relative path. It returns a path as an `&'static str` prefixed with the
    /// path to the crate root. Use `Path::new(relative!($path))` to retrieve an
    /// `&'static Path`.
    ///
    /// # Example
    ///
    /// Serve files from the crate-relative `static/` directory:
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::fs::{FileServer, relative};
    ///
    /// #[launch]
    /// fn rocket() -> _ {
    ///     rocket::build().mount("/", FileServer::new(relative!("static")))
    /// }
    /// ```
    ///
    /// Path equivalences:
    ///
    /// ```rust
    /// use std::path::Path;
    ///
    /// use rocket::fs::relative;
    ///
    /// let manual = Path::new(env!("CARGO_MANIFEST_DIR")).join("static");
    /// let automatic_1 = Path::new(relative!("static"));
    /// let automatic_2 = Path::new(relative!("/static"));
    /// assert_eq!(manual, automatic_1);
    /// assert_eq!(automatic_1, automatic_2);
    /// ```
    ///
    macro_rules! relative {
        ($path:expr) => {
            if cfg!(windows) {
                concat!(env!("CARGO_MANIFEST_DIR"), "\\", $path)
            } else {
                concat!(env!("CARGO_MANIFEST_DIR"), "/", $path)
            }
        };
    }
}
