//! Custom handler and options for static file serving.
//!
//! See the [`StaticFiles`](serve::StaticFiles) type for further details.
//!
//! # Enabling
//!
//! This module is only available when the `serve` feature is enabled. Enable it
//! in `Cargo.toml` as follows:
//!
//! ```toml
//! [dependencies.rocket_contrib]
//! version = "0.4.0"
//! default-features = false
//! features = ["serve"]
//! ```

use std::path::{PathBuf, Path};

use rocket::{Request, Data, Route};
use rocket::http::{Method, Status, uri::Segments};
use rocket::handler::{Handler, Outcome};
use rocket::response::NamedFile;
use rocket::outcome::IntoOutcome;

/// A bitset representing configurable options for the [`StaticFiles`] handler.
///
/// The valid options are:
///
///   * [`Options::None`] - Return only present, visible files.
///   * [`Options::DotFiles`] - In addition to visible files, return dotfiles.
///   * [`Options::Index`] - Render `index.html` pages for directory requests.
///
/// Two `Options` structures can be `or`d together to slect two or more options.
/// For instance, to request that both dot files and index pages be returned,
/// use `Options::DotFiles | Options::Index`.
#[derive(Debug, Clone, Copy)]
pub struct Options(u8);

#[allow(non_upper_case_globals)]
impl Options {
    /// `Options` representing the empty set. No dotfiles or index pages are
    /// rendered. This is different than the _default_, which enables `Index`.
    pub const None: Options = Options(0b0000);

    /// `Options` enabling responding to requests for a directory with the
    /// `index.html` file in that directory, if it exists. When this is enabled,
    /// the [`StaticFiles`] handler will respond to requests for a directory
    /// `/foo` with the file `${root}/foo/index.html` if it exists. This is
    /// enabled by default.
    pub const Index: Options = Options(0b0001);

    /// `Options` enabling returning dot files. When this is enabled, the
    /// [`StaticFiles`] handler will respond to requests for files or
    /// directories beginning with `.`. This is _not_ enabled by default.
    pub const DotFiles: Options = Options(0b0010);

    /// Returns `true` if `self` is a superset of `other`. In other words,
    /// returns `true` if all of the options in `other` are also in `self`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket_contrib::serve::Options;
    ///
    /// let index_request = Options::Index | Options::DotFiles;
    /// assert!(index_request.contains(Options::Index));
    /// assert!(index_request.contains(Options::DotFiles));
    ///
    /// let index_only = Options::Index;
    /// assert!(index_only.contains(Options::Index));
    /// assert!(!index_only.contains(Options::DotFiles));
    ///
    /// let dot_only = Options::DotFiles;
    /// assert!(dot_only.contains(Options::DotFiles));
    /// assert!(!dot_only.contains(Options::Index));
    /// ```
    #[inline]
    pub fn contains(self, other: Options) -> bool {
        (other.0 & self.0) == other.0
    }
}

impl ::std::ops::BitOr for Options {
    type Output = Self;

    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self {
        Options(self.0 | rhs.0)
    }
}

/// Custom handler for serving static files.
///
/// This handler makes it simple to serve static files from a directory on the
/// local file system. To use it, construct a `StaticFiles` using either
/// [`StaticFiles::from()`] or [`StaticFiles::new()`] then simply `mount` the
/// handler at a desired path.
///
/// # Options
///
/// The handler's functionality can be customized by passing an [`Options`] to
/// [`StaticFiles::new()`].
///
/// # Example
///
/// To serve files from the `/static` directory at the `/public` path, allowing
/// `index.html` files to be used to respond to requests for a directory (the
/// default), you might write the following:
///
/// ```rust
/// # extern crate rocket;
/// # extern crate rocket_contrib;
/// use rocket_contrib::serve::StaticFiles;
///
/// fn main() {
/// # if false {
///     rocket::ignite()
///         .mount("/public", StaticFiles::from("/static"))
///         .launch();
/// # }
/// }
/// ```
///
/// With this set-up, requests for files at `/public/<path..>` will be handled
/// by returning the contents of `/static/<path..>`. Requests for _directories_
/// at `/public/<directory>` will be handled by returning the contents of
/// `/static/<directory>/index.html`.
///
/// If your static files are stored relative to your crate and your project is
/// managed by Cargo, you should either use a relative path and ensure that your
/// server is started in the crate's root directory or use the
/// `CARGO_MANIFEST_DIR` to create an absolute path relative to your crate root.
/// For example, to serve files in the `static` subdirectory of your crate at
/// `/`, you might write:
///
/// ```rust
/// # extern crate rocket;
/// # extern crate rocket_contrib;
/// use rocket_contrib::serve::StaticFiles;
///
/// fn main() {
/// # if false {
///     rocket::ignite()
///         .mount("/", StaticFiles::from(concat!(env!("CARGO_MANIFEST_DIR"), "/static")))
///         .launch();
/// # }
/// }
/// ```
#[derive(Clone)]
pub struct StaticFiles {
    root: PathBuf,
    options: Options,
}

impl StaticFiles {
    /// Constructs a new `StaticFiles` that serves files from the file system
    /// `path`. By default, [`Options::Index`] is enabled. To serve static files
    /// with other options, use [`StaticFiles::new()`].
    ///
    /// # Example
    ///
    /// Serve the static files in the `/www/public` local directory on path
    /// `/static`.
    ///
    /// ```rust
    /// # extern crate rocket;
    /// # extern crate rocket_contrib;
    /// use rocket_contrib::serve::StaticFiles;
    ///
    /// fn main() {
    /// # if false {
    ///     rocket::ignite()
    ///         .mount("/static", StaticFiles::from("/www/public"))
    ///         .launch();
    /// # }
    /// }
    /// ```
    pub fn from<P: AsRef<Path>>(path: P) -> Self {
        StaticFiles::new(path, Options::Index)
    }

    /// Constructs a new `StaticFiles` that serves files from the file system
    /// `path` with `options` enabled.
    ///
    /// # Example
    ///
    /// Serve the static files in the `/www/public` local directory on path
    /// `/static` without serving index files or dot files. Additionally, serve
    /// the same files on `/pub` while also seriving index files and dot files.
    ///
    /// ```rust
    /// # extern crate rocket;
    /// # extern crate rocket_contrib;
    /// use rocket_contrib::serve::{StaticFiles, Options};
    ///
    /// fn main() {
    /// # if false {
    ///     let options = Options::Index | Options::DotFiles;
    ///     rocket::ignite()
    ///         .mount("/static", StaticFiles::from("/www/public"))
    ///         .mount("/pub", StaticFiles::new("/www/public", options))
    ///         .launch();
    /// # }
    /// }
    /// ```
    pub fn new<P: AsRef<Path>>(path: P, options: Options) -> Self {
        StaticFiles { root: path.as_ref().into(), options }
    }
}

impl Into<Vec<Route>> for StaticFiles {
    fn into(self) -> Vec<Route> {
        let non_index = Route::ranked(10, Method::Get, "/<path..>", self.clone());
        if self.options.contains(Options::Index) {
            let index = Route::ranked(10, Method::Get, "/", self);
            vec![index, non_index]
        } else {
            vec![non_index]
        }
    }
}

impl Handler for StaticFiles {
    fn handle<'r>(&self, req: &'r Request, _: Data) -> Outcome<'r> {
        fn handle_index<'r>(opt: Options, r: &'r Request, path: &Path) -> Outcome<'r> {
            if !opt.contains(Options::Index) {
                return Outcome::failure(Status::NotFound);
            }

            Outcome::from(r, NamedFile::open(path.join("index.html")).ok())
        }

        // If this is not the route with segments, handle it only if the user
        // requested a handling of index files.
        let current_route = req.route().expect("route while handling");
        let is_segments_route = current_route.uri.path().ends_with(">");
        if !is_segments_route {
            return handle_index(self.options, req, &self.root);
        }

        // Otherwise, we're handling segments. Get the segments as a `PathBuf`,
        // only allowing dotfiles if the user allowed it.
        let allow_dotfiles = self.options.contains(Options::DotFiles);
        let path = req.get_segments::<Segments>(0)
            .and_then(|res| res.ok())
            .and_then(|segments| segments.into_path_buf(allow_dotfiles).ok())
            .map(|path| self.root.join(path))
            .into_outcome(Status::NotFound)?;

        if path.is_dir() {
            handle_index(self.options, req, &path)
        } else {
            Outcome::from(req, NamedFile::open(&path).ok())
        }
    }
}
