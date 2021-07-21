use std::path::{PathBuf, Path};

use crate::{Request, Data};
use crate::http::{Method, uri::Segments, ext::IntoOwned};
use crate::route::{Route, Handler, Outcome};
use crate::response::Redirect;
use crate::fs::NamedFile;

/// Custom handler for serving static files.
///
/// This handler makes it simple to serve static files from a directory on the
/// local file system. To use it, construct a `FileServer` using either
/// [`FileServer::from()`] or [`FileServer::new()`] then simply `mount` the
/// handler at a desired path. When mounted, the handler will generate route(s)
/// that serve the desired static files. If a requested file is not found, the
/// routes _forward_ the incoming request. The default rank of the generated
/// routes is `10`. To customize route ranking, use the [`FileServer::rank()`]
/// method.
///
/// # Options
///
/// The handler's functionality can be customized by passing an [`Options`] to
/// [`FileServer::new()`].
///
/// # Example
///
/// To serve files from the `/static` directory on the local file system at the
/// `/public` path, allowing `index.html` files to be used to respond to
/// requests for a directory (the default), you might write the following:
///
/// ```rust,no_run
/// # #[macro_use] extern crate rocket;
/// use rocket::fs::FileServer;
///
/// #[launch]
/// fn rocket() -> _ {
///     rocket::build().mount("/public", FileServer::from("/static"))
/// }
/// ```
///
/// With this, requests for files at `/public/<path..>` will be handled by
/// returning the contents of `/static/<path..>`. Requests for _directories_ at
/// `/public/<directory>` will be handled by returning the contents of
/// `/static/<directory>/index.html`.
///
/// ## Relative Paths
///
/// In the example above, `/static` is an absolute path. If your static files
/// are stored relative to your crate and your project is managed by Rocket, use
/// the [`relative!`] macro to obtain a path that is relative to your
/// crate's root. For example, to serve files in the `static` subdirectory of
/// your crate at `/`, you might write:
///
/// ```rust,no_run
/// # #[macro_use] extern crate rocket;
/// use rocket::fs::{FileServer, relative};
///
/// #[launch]
/// fn rocket() -> _ {
///     rocket::build().mount("/", FileServer::from(relative!("static")))
/// }
/// ```
#[derive(Debug, Clone)]
pub struct FileServer {
    root: PathBuf,
    options: Options,
    rank: isize,
}

impl FileServer {
    /// The default rank use by `FileServer` routes.
    const DEFAULT_RANK: isize = 10;

    /// Constructs a new `FileServer` that serves files from the file system
    /// `path`. By default, [`Options::Index`] is set, and the generated routes
    /// have a rank of `10`. To serve static files with other options, use
    /// [`FileServer::new()`]. To choose a different rank for generated routes,
    /// use [`FileServer::rank()`].
    ///
    /// # Panics
    ///
    /// Panics if `path` does not exist or is not a directory.
    ///
    /// # Example
    ///
    /// Serve the static files in the `/www/public` local directory on path
    /// `/static`.
    ///
    /// ```rust,no_run
    /// # #[macro_use] extern crate rocket;
    /// use rocket::fs::FileServer;
    ///
    /// #[launch]
    /// fn rocket() -> _ {
    ///     rocket::build().mount("/static", FileServer::from("/www/public"))
    /// }
    /// ```
    ///
    /// Exactly as before, but set the rank for generated routes to `30`.
    ///
    /// ```rust,no_run
    /// # #[macro_use] extern crate rocket;
    /// use rocket::fs::FileServer;
    ///
    /// #[launch]
    /// fn rocket() -> _ {
    ///     rocket::build().mount("/static", FileServer::from("/www/public").rank(30))
    /// }
    /// ```
    #[track_caller]
    pub fn from<P: AsRef<Path>>(path: P) -> Self {
        FileServer::new(path, Options::default())
    }

    /// Constructs a new `FileServer` that serves files from the file system
    /// `path` with `options` enabled. By default, the handler's routes have a
    /// rank of `10`. To choose a different rank, use [`FileServer::rank()`].
    ///
    /// # Panics
    ///
    /// If [`Options::Missing`] is not set, panics if `path` does not exist or
    /// is not a directory. Otherwise does not panic.
    ///
    /// # Example
    ///
    /// Serve the static files in the `/www/public` local directory on path
    /// `/static` without serving index files or dot files. Additionally, serve
    /// the same files on `/pub` with a route rank of -1 while also serving
    /// index files and dot files.
    ///
    /// ```rust,no_run
    /// # #[macro_use] extern crate rocket;
    /// use rocket::fs::{FileServer, Options};
    ///
    /// #[launch]
    /// fn rocket() -> _ {
    ///     let options = Options::Index | Options::DotFiles;
    ///     rocket::build()
    ///         .mount("/static", FileServer::from("/www/public"))
    ///         .mount("/pub", FileServer::new("/www/public", options).rank(-1))
    /// }
    /// ```
    #[track_caller]
    pub fn new<P: AsRef<Path>>(path: P, options: Options) -> Self {
        use crate::yansi::Paint;

        let path = path.as_ref();
        if !options.contains(Options::Missing) {
            if !options.contains(Options::IndexFile) && !path.is_dir() {
                let path = path.display();
                error!("FileServer path '{}' is not a directory.", Paint::white(path));
                warn_!("Aborting early to prevent inevitable handler failure.");
                panic!("invalid directory: refusing to continue");
            } else if !path.exists() {
                let path = path.display();
                error!("FileServer path '{}' is not a file.", Paint::white(path));
                warn_!("Aborting early to prevent inevitable handler failure.");
                panic!("invalid file: refusing to continue");
            }
        }

        FileServer { root: path.into(), options, rank: Self::DEFAULT_RANK }
    }

    /// Sets the rank for generated routes to `rank`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rocket::fs::{FileServer, Options};
    ///
    /// // A `FileServer` created with `from()` with routes of rank `3`.
    /// FileServer::from("/public").rank(3);
    ///
    /// // A `FileServer` created with `new()` with routes of rank `-15`.
    /// FileServer::new("/public", Options::Index).rank(-15);
    /// ```
    pub fn rank(mut self, rank: isize) -> Self {
        self.rank = rank;
        self
    }
}

impl From<FileServer> for Vec<Route> {
    fn from(server: FileServer) -> Self {
        let source = figment::Source::File(server.root.clone());
        let mut route = Route::ranked(server.rank, Method::Get, "/<path..>", server);
        route.name = Some(format!("FileServer: {}", source).into());
        vec![route]
    }
}

#[crate::async_trait]
impl Handler for FileServer {
    async fn handle<'r>(&self, req: &'r Request<'_>, data: Data<'r>) -> Outcome<'r> {
        use crate::http::uri::fmt::Path;

        // TODO: Should we reject dotfiles for `self.root` if !DotFiles?
        let options = self.options;
        if options.contains(Options::IndexFile) && self.root.is_file() {
            let segments = match req.segments::<Segments<'_, Path>>(0..) {
                Ok(segments) => segments,
                Err(never) => match never {},
            };

            if segments.is_empty() {
                let file = NamedFile::open(&self.root).await.ok();
                return Outcome::from_or_forward(req, data, file);
            } else {
                return Outcome::forward(data);
            }
        }

        // Get the segments as a `PathBuf`, allowing dotfiles requested.
        let allow_dotfiles = options.contains(Options::DotFiles);
        let path = req.segments::<Segments<'_, Path>>(0..).ok()
            .and_then(|segments| segments.to_path_buf(allow_dotfiles).ok())
            .map(|path| self.root.join(path));

        match path {
            Some(p) if p.is_dir() => {
                // Normalize '/a/b/foo' to '/a/b/foo/'.
                if options.contains(Options::NormalizeDirs) && !req.uri().path().ends_with('/') {
                    let normal = req.uri().map_path(|p| format!("{}/", p))
                        .expect("adding a trailing slash to a known good path => valid path")
                        .into_owned();

                    return Outcome::from_or_forward(req, data, Redirect::permanent(normal));
                }

                if !options.contains(Options::Index) {
                    return Outcome::forward(data);
                }

                let index = NamedFile::open(p.join("index.html")).await.ok();
                Outcome::from_or_forward(req, data, index)
            },
            Some(p) => Outcome::from_or_forward(req, data, NamedFile::open(p).await.ok()),
            None => Outcome::forward(data),
        }
    }
}

/// A bitset representing configurable options for [`FileServer`].
///
/// The valid options are:
///
///   * [`Options::None`] - Return only present, visible files.
///   * [`Options::DotFiles`] - In addition to visible files, return dotfiles.
///   * [`Options::Index`] - Render `index.html` pages for directory requests.
///   * [`Options::IndexFile`] - Allow serving a single file as the index.
///   * [`Options::Missing`] - Don't fail if the path to serve is missing.
///   * [`Options::NormalizeDirs`] - Redirect directories without a trailing
///     slash to ones with a trailing slash.
///
/// `Options` structures can be `or`d together to select two or more options.
/// For instance, to request that both dot files and index pages be returned,
/// use `Options::DotFiles | Options::Index`.
#[derive(Debug, Clone, Copy)]
pub struct Options(u8);

#[allow(non_upper_case_globals, non_snake_case)]
impl Options {
    /// All options disabled.
    ///
    /// This is different than [`Options::default()`](#impl-Default), which
    /// enables `Options::Index`.
    pub const None: Options = Options(0);

    /// Respond to requests for a directory with the `index.html` file in that
    /// directory, if it exists.
    ///
    /// When enabled, [`FileServer`] will respond to requests for a directory
    /// `/foo` or `/foo/` with the file at `${root}/foo/index.html` if it
    /// exists. When disabled, requests to directories will always forward.
    ///
    /// **Enabled by default.**
    pub const Index: Options = Options(1 << 0);

    /// Allow serving dotfiles.
    ///
    /// When enabled, [`FileServer`] will respond to requests for files or
    /// directories beginning with `.`. When disabled, any dotfiles will be
    /// treated as missing.
    ///
    /// **Disabled by default.**
    pub const DotFiles: Options = Options(1 << 1);

    /// Normalizes directory requests by redirecting requests to directory paths
    /// without a trailing slash to ones with a trailing slash.
    ///
    /// When enabled, the [`FileServer`] handler will respond to requests for a
    /// directory without a trailing `/` with a permanent redirect (308) to the
    /// same path with a trailing `/`. This ensures relative URLs within any
    /// document served from that directory will be interpreted relative to that
    /// directory rather than its parent.
    ///
    /// **Disabled by default.**
    ///
    /// # Example
    ///
    /// Given the following directory structure...
    ///
    /// ```text
    /// static/
    /// └── foo/
    ///     ├── cat.jpeg
    ///     └── index.html
    /// ```
    ///
    /// ...with `FileServer::from("static")`, both requests to `/foo` and
    /// `/foo/` will serve `static/foo/index.html`. If `index.html` references
    /// `cat.jpeg` as a relative URL, the browser will request `/cat.jpeg`
    /// (`static/cat.jpeg`) when the request for `/foo` was handled and
    /// `/foo/cat.jpeg` (`static/foo/cat.jpeg`) if `/foo/` was handled. As a
    /// result, the request in the former case will fail. To avoid this,
    /// `NormalizeDirs` will redirect requests to `/foo` to `/foo/` if the file
    /// that would be served is a directory.
    pub const NormalizeDirs: Options = Options(1 << 2);

    /// Allow serving a file instead of a directory.
    ///
    /// By default, `FileServer` will error on construction if the path to serve
    /// does not point to a directory. When this option is enabled, if a path to
    /// a file is provided, `FileServer` will serve the file as the root of the
    /// mount path.
    ///
    /// # Example
    ///
    /// If the file tree looks like:
    ///
    /// ```text
    /// static/
    /// └── cat.jpeg
    /// ```
    ///
    /// Then `cat.jpeg` can be served at `/cat` with:
    ///
    /// ```rust,no_run
    /// # #[macro_use] extern crate rocket;
    /// use rocket::fs::{FileServer, Options};
    ///
    /// #[launch]
    /// fn rocket() -> _ {
    ///     rocket::build()
    ///         .mount("/cat", FileServer::new("static/cat.jpeg", Options::IndexFile))
    /// }
    /// ```
    pub const IndexFile: Options = Options(1 << 3);

    /// Don't fail if the file or directory to serve is missing.
    ///
    /// By default, `FileServer` will error if the path to serve is missing to
    /// prevent inevitable 404 errors. This option overrides that.
    pub const Missing: Options = Options(1 << 4);

    /// Returns `true` if `self` is a superset of `other`. In other words,
    /// returns `true` if all of the options in `other` are also in `self`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::fs::Options;
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

/// The default set of options: `Options::Index`.
impl Default for Options {
    fn default() -> Self {
        Options::Index
    }
}

impl std::ops::BitOr for Options {
    type Output = Self;

    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self {
        Options(self.0 | rhs.0)
    }
}

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
    ///     rocket::build().mount("/", FileServer::from(relative!("static")))
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
