use core::fmt;
use std::any::{type_name, Any, TypeId};
use std::path::{PathBuf, Path};
use std::sync::Arc;

use crate::{Data, Request};
use crate::http::{Method, Status, uri::{Segments, Reference}, ext::IntoOwned, HeaderMap};
use crate::route::{Route, Handler, Outcome};
use crate::response::{Redirect, Responder};
use crate::outcome::IntoOutcome;
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
/// Serve files from the `/static` directory on the local file system at the
/// `/public` path with the [default options](#impl-Default):
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
/// Requests for files at `/public/<path..>` will be handled by returning the
/// contents of `/static/<path..>`. Requests for _directories_ at
/// `/public/<directory>` will be handled by returning the contents of
/// `/static/<directory>/index.html`.
///
/// ## Relative Paths
///
/// In the example above, `/static` is an absolute path. If your static files
/// are stored relative to your crate and your project is managed by Cargo, use
/// the [`relative!`] macro to obtain a path that is relative to your crate's
/// root. For example, to serve files in the `static` subdirectory of your crate
/// at `/`, you might write:
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
#[derive(Clone)]
pub struct FileServer {
    root: PathBuf,
    options: Options,
    // TODO: I'd prefer box, but this just makes Clone easier.
    rewrites: Vec<Arc<dyn Rewrite>>,
    rank: isize,
}

impl fmt::Debug for FileServer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileServer")
            .field("root", &self.root)
            .field("options", &self.options)
            .field("rewrites", &DebugListRewrite(&self.rewrites))
            .field("rank", &self.rank)
            .finish()
    }
}

struct DebugListRewrite<'a>(&'a Vec<Arc<dyn Rewrite>>);

impl fmt::Debug for DebugListRewrite<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.0.iter().map(|r| r.name())).finish()
    }
}

pub trait Rewrite: Send + Sync + Any {
    /// Modify RewritablePath as needed.
    fn rewrite(&self, req: &Request<'_>, path: FileServerResponse, root: &Path)
        -> FileServerResponse;
    /// Allow multiple of the same rewrite
    fn allow_multiple(&self) -> bool { false }
    /// provides type name for debug printing
    fn name(&self) -> &'static str { type_name::<Self>() }
}

#[derive(Debug)]
pub enum HiddenReason {
    DoesNotExist,
    DotFile,
    PermissionDenied,
    Other,
}

#[derive(Debug)]
pub enum FileServerResponse {
    /// Status: Ok
    File {
        path: PathBuf,
        headers: HeaderMap<'static>,
    },
    /// Status: NotFound
    NotFound { path: PathBuf, reason: HiddenReason },
    /// Status: Redirect
    PermanentRedirect { to: Reference<'static> },
    /// Status: Redirect (TODO: should we allow this?)
    TemporaryRedirect { to: Reference<'static> },
}

/// Rewrites the path to allow paths that include a component beginning with a
/// a `.`. This rewrite should be applied first, before any other rewrite.
pub struct DotFiles;
impl Rewrite for DotFiles {
    fn rewrite(&self, _req: &Request<'_>, path: FileServerResponse, _root: &Path)
        -> FileServerResponse
    {
        match path {
            FileServerResponse::NotFound { path: name, reason: HiddenReason::DotFile } =>
                FileServerResponse::File { path: name, headers: HeaderMap::new() },
            path => path,
        }
    }
}
// struct Missing; // This only applies on startup, so this needs to be an option
// impl Rewrite for Missing {
//     fn rewrite(&self, req: &Request<'_>, path: &mut RewritablePath<'_>) {
//         todo!()
//     }
// }

/// Rewrites a path to a directory to return the content of an index file, `index.html`
/// by defualt.
///
/// # Examples
/// - Rewrites `/` to `/index.html`
/// - Rewrites `/home/` to `/home/index.html`
/// - Does not rewrite `/home/test.html`
pub struct Index(pub &'static str);
impl Rewrite for Index {
    fn rewrite(&self, _req: &Request<'_>, path: FileServerResponse, _root: &Path)
        -> FileServerResponse
    {
        match path {
            FileServerResponse::File { path: name, headers } if name.is_dir() =>
                FileServerResponse::File { path: name.join(self.0), headers },
            path => path,
        }
    }
}
impl Default for Index {
    fn default() -> Self {
        Self("index.html")
    }
}


// Actually, curiously, this already just works as-is (the only thing that prevents
// it is the startup check)
pub struct IndexFile;
impl Rewrite for IndexFile {
    fn rewrite(&self, _req: &Request<'_>, path: FileServerResponse, _root: &Path)
        -> FileServerResponse
    {
        match path {
            path => path,
        }
    }
}

/// Rewrites a path to a directory without a trailing slash to a redirect to
/// the same directory with a trailing slash. This rewrite needs to be applied
/// before [`Index`] and any other rewrite that changes the path to a file.
///
/// # Examples
/// - Redirects `/home/test` to `/home/test/`
/// - Does not redirect `/home/`
/// - Does not redirect `/home/index.html`
pub struct NormalizeDirs;
impl Rewrite for NormalizeDirs {
    fn rewrite(&self, req: &Request<'_>, path: FileServerResponse, _root: &Path)
        -> FileServerResponse
    {
        match path {
            FileServerResponse::File { path: name, .. } if !req.uri().path().ends_with('/') &&
                name.is_dir() =>
                FileServerResponse::PermanentRedirect {
                    to: req.uri().map_path(|p| format!("{}/", p))
                        .expect("adding a trailing slash to a known good path => valid path")
                        .into_owned().into()
                },
            path => path,
        }
    }
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
        FileServer::new(path, Options::None)
            .rewrite(NormalizeDirs)
            .rewrite(Index("index.html"))
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
        let path = path.as_ref();
        if !options.contains(Options::Missing) {
            #[allow(deprecated)]
            if !options.contains(Options::IndexFile) && !path.is_dir() {
                error!(path = %path.display(),
                    "FileServer path does not point to a directory.\n\
                    Aborting early to prevent inevitable handler runtime errors.");

                panic!("invalid directory path: refusing to continue");
            } else if !path.exists() {
                error!(path = %path.display(),
                    "FileServer path does not point to a file.\n\
                    Aborting early to prevent inevitable handler runtime errors.");

                panic!("invalid file path: refusing to continue");
            }
        }
        let mut rewrites: Vec<Arc<dyn Rewrite>> = vec![];
        #[allow(deprecated)]
        if options.contains(Options::DotFiles) {
            rewrites.push(Arc::new(DotFiles));
        }
        #[allow(deprecated)]
        if options.contains(Options::NormalizeDirs) {
            rewrites.push(Arc::new(NormalizeDirs));
        }
        #[allow(deprecated)]
        if options.contains(Options::Index) {
            rewrites.push(Arc::new(Index("index.html")));
        }

        FileServer { root: path.into(), options, rewrites, rank: Self::DEFAULT_RANK }
    }

    /// Removes all rewrites of a specific type.
    ///
    /// Ideally, this shouldn't exist, and it should be possible to always just not add
    /// the rewrites you don't want.
    pub fn remove_rewrites<T: Rewrite>(mut self) -> Self {
        self.rewrites.retain(|r| r.as_ref().type_id() != TypeId::of::<T>());
        self
    }

    /// Add a rewrite step to this `FileServer`. The order in which rewrites are added can make
    /// a difference, since they are applied in the order they appear.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[macro_use] extern crate rocket;
    /// use rocket::fs::{FileServer, Options, NormalizeDirs, Index};
    ///
    /// #[launch]
    /// fn rocket() -> _ {
    ///     rocket::build()
    ///         .mount("/static", FileServer::from("/www/public"))
    ///         .mount("/pub",
    ///             FileServer::new("/www/public", Options::None)
    ///                 .rewrite(NormalizeDirs)
    ///                 .rewrite(Index::default())
    ///         )
    /// }
    /// ```
    /// In this example, order actually does matter. [`NormalizeDirs`] will convert a path to a
    /// directory without a trailing slash into a redirect to the same path, but with the trailing
    /// slash. However, if the [`Index`] rewrite is applied first, the path will have been changed
    /// to the `index.html` file, causing [`NormalizeDirs`] to do nothing.
    pub fn rewrite(mut self, rewrite: impl Rewrite) -> Self {
        if rewrite.allow_multiple() ||
            !self.rewrites.iter().any(|f| f.as_ref().type_id() == rewrite.type_id()) {
            self.rewrites.push(Arc::new(rewrite));
        } else {
            error!(
                "Attempted to insert multiple of the same rewrite `{}` on a FileServer.\n\
                Adding a rewrite that doesn't support duplicate rewrites is ignored",
                rewrite.name()
            );
        }
        self
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
        use crate::http::uri::fmt::Path as UriPath;
        let path = req.segments::<Segments<'_, UriPath>>(0..).ok()
            .and_then(|segments| segments.to_path_buf_dotfiles().ok())
            .map(|(path, dots)| (self.root.join(path), dots));
        let mut response = match path {
            Some((path, false)) =>
                FileServerResponse::File { path, headers: HeaderMap::new() },
            Some((path, true)) =>
                FileServerResponse::NotFound { path, reason: HiddenReason::DotFile },
            None => return Outcome::forward(data, Status::NotFound),
        };
        // println!("initial: {response:?}");
        for rewrite in &self.rewrites {
            response = rewrite.rewrite(req, response, &self.root);
            // println!("after: {} {response:?}", rewrite.name());
        }
        // Open TODOs:
        // - Should we validate the location of the path? We've aleady removed any
        //   `..` and other traversal mechanisms, before passing to the rewrites.
        // - Should we allow more control? I think we should require the payload
        //   to be a file on the disk, and the only thing left to control would be
        //   headers (which we allow configuring), and status, which we allow a specific
        //   set.
        // - Should we prepend the path root? I think we should, since I don't think the
        //   relative path is particularly useful.
        match response {
            FileServerResponse::File { path, headers } => {
                if path.is_dir() {
                    return Outcome::Forward((data, Status::NotFound));
                }
                NamedFile::open(path).await.respond_to(req).map(|mut r| {
                    for header in headers {
                        r.adjoin_raw_header(header.name.as_str().to_owned(), header.value);
                    }
                    r
                }).or_forward((data, Status::NotFound))
            },
            FileServerResponse::NotFound { .. } => Outcome::forward(data, Status::NotFound),
            FileServerResponse::PermanentRedirect { to } => Redirect::permanent(to)
                .respond_to(req)
                .or_forward((data, Status::InternalServerError)),
            FileServerResponse::TemporaryRedirect { to } => Redirect::temporary(to)
                .respond_to(req)
                .or_forward((data, Status::InternalServerError)),
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
    /// Note that this is different than [`Options::default()`](#impl-Default),
    /// which enables options.
    pub const None: Options = Options(0);

    /// Respond to requests for a directory with the `index.html` file in that
    /// directory, if it exists.
    ///
    /// When enabled, [`FileServer`] will respond to requests for a directory
    /// `/foo` or `/foo/` with the file at `${root}/foo/index.html` if it
    /// exists. When disabled, requests to directories will always forward.
    ///
    /// **Enabled by default.**
    #[deprecated(note = "Replaced by `.rewrite(Index(\"index.html\"))`")]
    pub const Index: Options = Options(1 << 0);

    /// Allow serving dotfiles.
    ///
    /// When enabled, [`FileServer`] will respond to requests for files or
    /// directories beginning with `.`. When disabled, any dotfiles will be
    /// treated as missing.
    ///
    /// **Disabled by default.**
    #[deprecated(note = "Replaced by `.rewrite(DotFiles)`")]
    pub const DotFiles: Options = Options(1 << 1);

    /// Normalizes directory requests by redirecting requests to directory paths
    /// without a trailing slash to ones with a trailing slash.
    ///
    /// **Enabled by default.**
    ///
    /// When enabled, the [`FileServer`] handler will respond to requests for a
    /// directory without a trailing `/` with a permanent redirect (308) to the
    /// same path with a trailing `/`. This ensures relative URLs within any
    /// document served from that directory will be interpreted relative to that
    /// directory rather than its parent.
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
    /// And the following server:
    ///
    /// ```text
    /// rocket.mount("/", FileServer::from("static"))
    /// ```
    ///
    /// ...requests to `example.com/foo` will be redirected to
    /// `example.com/foo/`. If `index.html` references `cat.jpeg` as a relative
    /// URL, the browser will resolve the URL to `example.com/foo/cat.jpeg`,
    /// which in-turn Rocket will match to `/static/foo/cat.jpg`.
    ///
    /// Without this option, requests to `example.com/foo` would not be
    /// redirected. `index.html` would be rendered, and the relative link to
    /// `cat.jpeg` would be resolved by the browser as `example.com/cat.jpeg`.
    /// Rocket would thus try to find `/static/cat.jpeg`, which does not exist.
    #[deprecated(note = "Replaced by `.rewrite(NormalizeDirs)`")]
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

/// The default set of options: `Options::Index | Options:NormalizeDirs`.
impl Default for Options {
    #[allow(deprecated)]
    fn default() -> Self {
        Options::Index | Options::NormalizeDirs
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
