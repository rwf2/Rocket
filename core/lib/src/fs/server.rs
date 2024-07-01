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
/// [`FileServer::new()`], then simply `mount` the handler. When mounted, the
/// handler serves files from the specified directory. If the file is not found,
/// the handler _forwards_ the request. By default, `FileServer` has a rank of
/// `10`. Use [`FileServer::new()`] to create a handler with a custom rank.
///
/// # Customization
///
/// How `FileServer` responds to specific requests can be customized through
/// the use of [`Rewriter`]s. See [`Rewriter`] for more detailed documentation
/// on how to take full advantage of `FileServer`'s extensibility.
///
/// [`FileServer::new()`] construct a `FileServer`
/// with common rewrites: they filter out dotfiles, redirect requests to
/// directories to include a trailing slash, and use `index.html` to respond to
/// requests for a directory. If you want to customize or replace these default
/// rewrites, see [`FileServer::empty()`].
///
/// # Example
///
/// Serve files from the `/static` directory on the local file system at the
/// `/public` path, with the default rewrites.
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
#[derive(Clone)]
pub struct FileServer {
    rewrites: Vec<Arc<dyn Rewriter>>,
    rank: isize,
}

impl FileServer {
    /// The default rank use by `FileServer` routes.
    const DEFAULT_RANK: isize = 10;

    /// Constructs a new `FileServer` that serves files from the file system
    /// `path` with a default rank.
    ///
    /// Adds a set of default rewrites:
    /// - `|f, _| f.is_visible()`: Hides all dotfiles.
    /// - [`Prefix::checked(path)`]: Applies the root path.
    /// - [`TrailingDirs`]: Normalizes directories to have a trailing slash.
    /// - [`DirIndex::unconditional("index.html")`](DirIndex::unconditional):
    ///   Appends `index.html` to directory requests.
    ///
    /// If you don't want to serve requests for directories, or want to
    /// customize what files are served when a directory is requested, see
    /// [`Self::new_without_index`].
    ///
    /// If you need to allow requests for dotfiles, or make any other changes
    /// to the default rewrites, see [`Self::empty`].
    ///
    /// [`Prefix::checked(path)`]: crate::fs::rewrite::Prefix::checked
    /// [`TrailingDirs`]: crate::fs::rewrite::TrailingDirs
    /// [`DirIndex::unconditional`]: crate::fs::DirIndex::unconditional
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self::empty()
            .filter(|f, _| f.is_visible())
            .rewrite(Prefix::checked(path))
            .rewrite(TrailingDirs)
            .rewrite(DirIndex::unconditional("index.html"))
    }

    /// Constructs a new `FileServer` that serves files from the file system
    /// `path` with a default rank. This variant does not add a default
    /// directory index option.
    ///
    /// Adds a set of default rewrites:
    /// - `|f, _| f.is_visible()`: Hides all dotfiles.
    /// - [`Prefix::checked(path)`]: Applies the root path.
    /// - [`TrailingDirs`]: Normalizes directories to have a trailing slash.
    ///
    /// In most cases, [`Self::new`] is good enough. However, if you do not want
    /// to automatically respond to requests for directories with `index.html`,
    /// this method is provided.
    ///
    /// # Example
    ///
    /// Constructs a default file server to  server files from `./static`, but
    /// uses `index.txt` if `index.html` doesn't exist.
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
    pub fn new_without_index<P: AsRef<Path>>(path: P) -> Self {
        Self::empty()
            .filter(|f, _| f.is_visible())
            .rewrite(Prefix::checked(path))
            .rewrite(TrailingDirs)
    }

    /// Constructs a new `FileServer`, with default rank, and no rewrites.
    pub fn empty() -> Self {
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
    /// FileServer::empty()
    ///    .rank(5)
    /// # }
    pub fn rank(mut self, rank: isize) -> Self {
        self.rank = rank;
        self
    }

    /// Generic rewrite to transform one Rewrite to another.
    ///
    /// # Example
    ///
    /// Redirects all requests that have been filtered to the root of the `FileServer`.
    ///
    /// ```rust,no_run
    /// # use rocket::fs::{FileServer, rewrite::Rewrite};
    /// # use rocket::{response::Redirect, uri, Build, Rocket, Request};
    /// fn redir_missing<'r>(p: Option<Rewrite<'r>>, _req: &Request<'_>) -> Option<Rewrite<'r>> {
    ///     match p {
    ///         None => Redirect::temporary(uri!("/")).into(),
    ///         p => p,
    ///     }
    /// }
    ///
    /// # fn launch() -> Rocket<Build> {
    /// rocket::build()
    ///     .mount("/", FileServer::new("static").rewrite(redir_missing))
    /// # }
    /// ```
    ///
    /// Note that `redir_missing` is not a closure in this example. Making it a closure
    /// causes compilation to fail with a lifetime error. It really shouldn't but it does.
    pub fn rewrite<R: Rewriter>(mut self, rewriter: R) -> Self {
        self.rewrites.push(Arc::new(rewriter));
        self
    }

    /// Filter what files this `FileServer` will respond with
    ///
    /// # Example
    ///
    /// Filter out all paths with a filename of `hidden`.
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

    /// Change what files this `FileServer` will respond with
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
