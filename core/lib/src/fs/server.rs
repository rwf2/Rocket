use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::{Data, Request};
use crate::outcome::IntoOutcome;
use crate::http::{uri::Segments, Method, Status};
use crate::route::{Route, Handler, Outcome};
use crate::response::Responder;
use crate::util::Formatter;
use crate::fs::rewrite::*;

/// Custom handler for serving static files.
///
/// This handler makes is simple to serve static files from a directory on the
/// local file system. To use it, construct a `FileServer` using
/// [`FileServer::from()`], then simply `mount` the handler. When mounted, the
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
/// [`FileServer::from()`] and [`FileServer::new()`] construct a `FileServer`
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
///     rocket::build().mount("/public", FileServer::from("/static"))
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
///     rocket::build().mount("/", FileServer::from(relative!("static")))
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
    /// - [`filter_dotfiles`]: Hides all dotfiles.
    /// - [`prefix(path)`](prefix): Applies the root path.
    /// - [`normalize_dirs`]: Normalizes directories to have a trailing slash.
    /// - [`index("index.html")`](index): Appends `index.html` to directory requests.
    pub fn from<P: AsRef<Path>>(path: P) -> Self {
        Self::empty()
            .filter(filter_dotfiles)
            .map(prefix(path))
            .map(normalize_dirs)
            .map(index("index.html"))
    }

    /// Constructs a new `FileServer`, with default rank, and no rewrites.
    ///
    /// See [`FileServer::empty_ranked()`].
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
    ///  }
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
    /// # use rocket::{Rocket, Build, Request};
    /// # use rocket::fs::{FileServer, Rewrite};
    /// # use rocket::{response::Redirect, #     uri, Build, Rocket, Request};
    /// fn redir_missing<'r>(p: Option<Rewrite<'r>>, _req: &Request<'_>) -> Option<Rewrite<'r>> {
    ///     match p {
    ///         None => Redirect::temporary(uri!("/")).into(),
    ///         p => p,
    ///     }
    /// }
    ///
    /// # fn launch() -> Rocket<Build> {
    /// rocket::build()
    ///     .mount("/", FileServer::from("static").rewrite(redir_missing))
    /// # }
    /// ```
    ///
    /// Note that `redir_missing` is not a closure in this example. Making it a closure
    /// causes compilation to fail with a lifetime error. It really shouldn't but it does.
    pub fn rewrite(mut self, f: impl Rewriter) -> Self {
        self.rewrites.push(Arc::new(f));
        self
    }

    /// Filter what files this `FileServer` will respond with
    ///
    /// # Example
    ///
    /// Filter out all paths with a filename of `hidden`.
    /// ```rust,no_run
    /// #[macro_use] extern crate rocket;
    /// use rocket::fs::FileServer;
    ///
    /// #[launch]
    /// fn rocket() -> _ {
    ///     let server = FileServer::from("static")
    ///         .filter(|f, _| f.path.file_name() != Some("hidden".as_ref()));
    ///
    ///     rocket::build()
    ///         .mount("/", server)
    /// # }
    /// ```
    pub fn filter<F>(self, f: F) -> Self
        where F: Fn(&File<'_>, &Request<'_>) -> bool + Send + Sync + 'static
    {
        struct FilterFile<F>(F);

        impl<F> Rewriter for FilterFile<F>
            where F: Fn(&File<'_>, &Request<'_>) -> bool + Send + Sync + 'static
        {
            fn rewrite<'r>(&self, f: Option<Rewrite<'r>>, r: &Request<'_>) -> Option<Rewrite<'r>> {
                match f {
                    Some(Rewrite::File(file)) if !self.0(&file, r) => None,
                    path => path,
                }
            }
        }

        self.rewrite(FilterFile(f))
    }

    /// Transform files
    ///
    /// # Example
    ///
    /// Append `hidden` to the path of every file returned.
    /// ```rust,no_run
    /// # use rocket::{fs::FileServer, Build, Rocket};
    /// # fn launch() -> Rocket<Build> {
    /// rocket::build()
    ///     .mount(
    ///         "/",
    ///         FileServer::from("static")
    ///             .map(|f, _r| f.map_path(|p| p.join("hidden")).into())
    ///     )
    /// # }
    /// ```
    pub fn map<F>(self, f: F) -> Self
        where F: for<'r> Fn(File<'r>, &Request<'_>) -> Rewrite<'r> + Send + Sync + 'static
    {
        struct MapFile<F>(F);

        impl<F> Rewriter for MapFile<F>
            where F: for<'r> Fn(File<'r>, &Request<'_>) -> Rewrite<'r> + Send + Sync + 'static,
        {
            fn rewrite<'r>(&self, f: Option<Rewrite<'r>>, r: &Request<'_>) -> Option<Rewrite<'r>> {
                match f {
                    Some(Rewrite::File(file)) => Some(self.0(file, r)),
                    path => path,
                }
            }
        }

        self.rewrite(MapFile(f))
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
