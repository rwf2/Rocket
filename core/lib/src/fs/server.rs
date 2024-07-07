use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::borrow::Cow;

use crate::{response, Data, Request, Response};
use crate::outcome::IntoOutcome;
use crate::http::{uri::Segments, HeaderMap, Method, ContentType, Status};
use crate::route::{Route, Handler, Outcome};
use crate::response::Responder;
use crate::util::Formatter;
use crate::fs::rewrite::*;

/// Custom handler for serving static files.
///
/// This handler makes is simple to serve static files from a directory on the
/// local file system. To use it, construct a `FileServer` using
/// [`FileServer::new()`], then `mount` the handler.
///
/// ```rust,no_run
/// # #[macro_use] extern crate rocket;
/// use rocket::fs::FileServer;
///
/// #[launch]
/// fn rocket() -> _ {
///     rocket::build()
///         .mount("/", FileServer::new("/www/static"))
/// }
/// ```
///
/// When mounted, the handler serves files from the specified path. If a
/// requested file does not exist, the handler _forwards_ the request with a
/// `404` status.
///
/// By default, the route has a rank of `10` which can be changed with
/// [`FileServer::rank()`].
///
/// # Customization
///
/// `FileServer` works through a pipeline of _rewrites_ in which a requested
/// path is transformed into a `PathBuf` via [`Segments::to_path_buf()`] and
/// piped through a series of [`Rewriter`]s to obtain a final [`Rewrite`] which
/// is then used to generate a final response. See [`Rewriter`] for complete
/// details on implementing your own `Rewriter`s.
///
/// # Example
///
/// Serve files from the `/static` directory on the local file system at the
/// `/public` path:
///
/// ```rust,no_run
/// # #[macro_use] extern crate rocket;
/// use rocket::fs::FileServer;
///
/// #[launch]
/// fn rocket() -> _ {
///     rocket::build().mount("/public", FileServer::new("/static"))
/// }
/// ```
///
/// Requests for files at `/public/<path..>` will be handled by returning the
/// contents of `/static/<path..>`. Requests for directories will return the
/// contents of `index.html`.
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
///     rocket::build().mount("/", FileServer::new(relative!("static")))
/// }
/// ```
///
/// [`relative!`]: crate::fs::relative!
#[derive(Clone)]
pub struct FileServer {
    rewrites: Vec<Arc<dyn Rewriter>>,
    rank: isize,
}

impl FileServer {
    /// The default rank use by `FileServer` routes.
    const DEFAULT_RANK: isize = 10;

    /// Constructs a new `FileServer` that serves files from the file system
    /// `path` with the following rewrites:
    ///
    /// - `|f, _| f.is_visible()`: Serve only visible files (hide dotfiles).
    /// - [`Prefix::checked(path)`]: Prefix requests with `path`.
    /// - [`TrailingDirs`]: Ensure directory have a trailing slash.
    /// - [`DirIndex::unconditional("index.html")`]: Serve `$dir/index.html` for
    ///   requests to directory `$dir`.
    ///
    /// If you don't want to serve index files or want a different index file,
    /// use [`Self::without_index`]. To customize the entire request to file
    /// path rewrite pipeline, use [`Self::identity`].
    ///
    /// [`Prefix::checked(path)`]: crate::fs::rewrite::Prefix::checked
    /// [`TrailingDirs`]: crate::fs::rewrite::TrailingDirs
    /// [`DirIndex::unconditional("index.html")`]: DirIndex::unconditional()
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[macro_use] extern crate rocket;
    /// use rocket::fs::FileServer;
    ///
    /// #[launch]
    /// fn rocket() -> _ {
    ///     rocket::build()
    ///         .mount("/", FileServer::new("/www/static"))
    /// }
    /// ```
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self::identity()
            .filter(|f, _| f.is_visible())
            .rewrite(Prefix::checked(path))
            .rewrite(TrailingDirs)
            .rewrite(DirIndex::unconditional("index.html"))
    }

    /// Exactly like [`FileServer::new()`] except it _does not_ serve directory
    /// index files via [`DirIndex`]. It rewrites with the following:
    ///
    /// - `|f, _| f.is_visible()`: Serve only visible files (hide dotfiles).
    /// - [`Prefix::checked(path)`]: Prefix requests with `path`.
    /// - [`TrailingDirs`]: Ensure directory have a trailing slash.
    ///
    /// # Example
    ///
    /// Constructs a default file server to serve files from `./static` using
    /// `index.txt` as the index file if `index.html` doesn't exist.
    ///
    /// ```rust,no_run
    /// # #[macro_use] extern crate rocket;
    /// use rocket::fs::{FileServer, rewrite::DirIndex};
    ///
    /// #[launch]
    /// fn rocket() -> _ {
    ///     let server = FileServer::new("static")
    ///         .rewrite(DirIndex::if_exists("index.html"))
    ///         .rewrite(DirIndex::unconditional("index.txt"));
    ///
    ///     rocket::build()
    ///         .mount("/", server)
    /// }
    /// ```
    ///
    /// [`Prefix::checked(path)`]: crate::fs::rewrite::Prefix::checked
    /// [`TrailingDirs`]: crate::fs::rewrite::TrailingDirs
    pub fn without_index<P: AsRef<Path>>(path: P) -> Self {
        Self::identity()
            .filter(|f, _| f.is_visible())
            .rewrite(Prefix::checked(path))
            .rewrite(TrailingDirs)
    }

    /// Constructs a new `FileServer` with no rewrites.
    ///
    /// Without any rewrites, a `FileServer` will try to serve the requested
    /// file from the current working directory. In other words, it represents
    /// the identity rewrite. For example, a request `GET /foo/bar` will be
    /// passed through unmodified and thus `./foo/bar` will be served. This is
    /// very unlikely to be what you want.
    ///
    /// Prefer to use [`FileServer::new()`] or [`FileServer::without_index()`]
    /// whenever possible and otherwise use one or more of the rewrites in
    /// [`rocket::fs::rewrite`] or your own custom rewrites.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[macro_use] extern crate rocket;
    /// use rocket::fs::{FileServer, rewrite};
    ///
    /// #[launch]
    /// fn rocket() -> _ {
    ///     // A file server that serves exactly one file: /www/foo.html. The
    ///     // file is served irrespective of what's requested.
    ///     let server = FileServer::identity()
    ///         .rewrite(rewrite::File::checked("/www/foo.html"));
    ///
    ///     rocket::build()
    ///         .mount("/", server)
    /// }
    /// ```
    pub fn identity() -> Self {
        Self {
            rewrites: vec![],
            rank: Self::DEFAULT_RANK
        }
    }

    /// Sets the rank of the route emitted by the `FileServer` to `rank`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rocket::fs::FileServer;
    /// # fn make_server() -> FileServer {
    /// FileServer::identity()
    ///    .rank(5)
    /// # }
    pub fn rank(mut self, rank: isize) -> Self {
        self.rank = rank;
        self
    }

    /// Add `rewriter` to the rewrite pipeline.
    ///
    /// # Example
    ///
    /// Redirect filtered requests (`None`) to `/`.
    ///
    /// ```rust,no_run
    /// # #[macro_use] extern crate rocket;
    /// use rocket::fs::{FileServer, rewrite::Rewrite};
    /// use rocket::{request::Request, response::Redirect};
    ///
    /// fn redir_missing<'r>(p: Option<Rewrite<'r>>, _req: &Request<'_>) -> Option<Rewrite<'r>> {
    ///     Some(p.unwrap_or_else(|| Redirect::temporary(uri!("/")).into()))
    /// }
    ///
    /// #[launch]
    /// fn rocket() -> _ {
    ///     rocket::build()
    ///         .mount("/", FileServer::new("static").rewrite(redir_missing))
    /// }
    /// ```
    ///
    /// Note that `redir_missing` is not a closure in this example. Making it a closure
    /// causes compilation to fail with a lifetime error. It really shouldn't but it does.
    pub fn rewrite<R: Rewriter>(mut self, rewriter: R) -> Self {
        self.rewrites.push(Arc::new(rewriter));
        self
    }

    /// Adds a rewriter to the pipeline that returns `Some` only when the
    /// function `f` returns `true`, filtering out all other files.
    ///
    /// # Example
    ///
    /// Allow all files that don't have a file name or have a file name other
    /// than "hidden".
    ///
    /// ```rust,no_run
    /// # #[macro_use] extern crate rocket;
    /// use rocket::fs::FileServer;
    ///
    /// #[launch]
    /// fn rocket() -> _ {
    ///     let server = FileServer::new("static")
    ///         .filter(|f, _| f.path.file_name() != Some("hidden".as_ref()));
    ///
    ///     rocket::build()
    ///         .mount("/", server)
    /// }
    /// ```
    pub fn filter<F: Send + Sync + 'static>(self, f: F) -> Self
        where F: Fn(&File<'_>, &Request<'_>) -> bool
    {
        struct Filter<F>(F);

        impl<F> Rewriter for Filter<F>
            where F: Fn(&File<'_>, &Request<'_>) -> bool + Send + Sync + 'static
        {
            fn rewrite<'r>(&self, f: Option<Rewrite<'r>>, r: &Request<'_>) -> Option<Rewrite<'r>> {
                f.and_then(|f| match f {
                    Rewrite::File(f) if self.0(&f, r) => Some(Rewrite::File(f)),
                    _ => None,
                })
            }
        }

        self.rewrite(Filter(f))
    }

    /// Adds a rewriter to the pipeline that maps the current `File` to another
    /// `Rewrite` using `f`. If the current `Rewrite` is a `Redirect`, it is
    /// passed through without calling `f`.
    ///
    /// # Example
    ///
    /// Append `index.txt` to every path.
    ///
    /// ```rust,no_run
    /// # #[macro_use] extern crate rocket;
    /// use rocket::fs::FileServer;
    ///
    /// #[launch]
    /// fn rocket() -> _ {
    ///     let server = FileServer::new("static")
    ///         .map(|f, _| f.map_path(|p| p.join("index.txt")).into());
    ///
    ///     rocket::build()
    ///         .mount("/", server)
    /// }
    /// ```
    pub fn map<F: Send + Sync + 'static>(self, f: F) -> Self
        where F: for<'r> Fn(File<'r>, &Request<'_>) -> Rewrite<'r>
    {
        struct Map<F>(F);

        impl<F> Rewriter for Map<F>
            where F: for<'r> Fn(File<'r>, &Request<'_>) -> Rewrite<'r> + Send + Sync + 'static
        {
            fn rewrite<'r>(&self, f: Option<Rewrite<'r>>, r: &Request<'_>) -> Option<Rewrite<'r>> {
                f.map(|f| match f {
                    Rewrite::File(f) => self.0(f, r),
                    Rewrite::Redirect(r) => Rewrite::Redirect(r),
                })
            }
        }

        self.rewrite(Map(f))
    }
}

impl From<FileServer> for Vec<Route> {
    fn from(server: FileServer) -> Self {
        let mut route = Route::ranked(server.rank, Method::Get, "/<path..>", server);
        route.name = Some("FileServer".into());
        vec![route]
    }
}

#[crate::async_trait]
impl Handler for FileServer {
    async fn handle<'r>(&self, req: &'r Request<'_>, data: Data<'r>) -> Outcome<'r> {
        use crate::http::uri::fmt::Path as UriPath;
        let path: Option<PathBuf> = req.segments::<Segments<'_, UriPath>>(0..).ok()
            .and_then(|segments| segments.to_path_buf(true).ok());

        let mut response = path.map(|p| Rewrite::File(File::new(p)));
        for rewrite in &self.rewrites {
            response = rewrite.rewrite(response, req);
        }

        let (outcome, status) = match response {
            Some(Rewrite::File(f)) => (f.open().await.respond_to(req), Status::NotFound),
            Some(Rewrite::Redirect(r)) => (r.respond_to(req), Status::InternalServerError),
            None => return Outcome::forward(data, Status::NotFound),
        };

        outcome.or_forward((data, status))
    }
}

impl fmt::Debug for FileServer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileServer")
            .field("rewrites", &Formatter(|f| write!(f, "<{} rewrites>", self.rewrites.len())))
            .field("rank", &self.rank)
            .finish()
    }
}

impl<'r> File<'r> {
    async fn open(self) -> std::io::Result<NamedFile<'r>> {
        let file = tokio::fs::File::open(&self.path).await?;
        let metadata = file.metadata().await?;
        if metadata.is_dir() {
            return Err(std::io::Error::other("is a directory"));
        }

        Ok(NamedFile {
            file,
            len: metadata.len(),
            path: self.path,
            headers: self.headers,
        })
    }
}

struct NamedFile<'r> {
    file: tokio::fs::File,
    len: u64,
    path: Cow<'r, Path>,
    headers: HeaderMap<'r>,
}

// Do we want to allow the user to rewrite the Content-Type?
impl<'r> Responder<'r, 'r> for NamedFile<'r> {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'r> {
        let mut response = Response::new();
        response.set_header_map(self.headers);
        if !response.headers().contains("Content-Type") {
            self.path.extension()
                .and_then(|ext| ext.to_str())
                .and_then(ContentType::from_extension)
                .map(|content_type| response.set_header(content_type));
        }

        response.set_sized_body(self.len as usize, self.file);
        Ok(response)
    }
}
