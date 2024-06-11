use core::fmt;
use std::borrow::Cow;
use std::ffi::OsStr;
use std::path::{Path, PathBuf, MAIN_SEPARATOR_STR};
use std::sync::Arc;

use crate::fs::NamedFile;
use crate::{Data, Request, outcome::IntoOutcome};
use crate::http::{
    Method,
    HeaderMap,
    Header,
    uri::Segments,
    Status,
    ext::IntoOwned,
};
use crate::route::{Route, Handler, Outcome};
use crate::response::{Redirect, Responder};

/// Custom handler for serving static files.
///
/// This handler makes is simple to serve static files from a directory on the
/// local file system. To use it, construct a `FileServer` using
/// [`FileServer::from()`], then simply `mount` the handler. When mounted, the
/// handler serves files from the specified directory. If the file is not found,
/// the handler _forwards_ the request. By default, `FileServer` has a rank of
/// `10`. Use [`FileServer::new()`] to create a route with a custom rank.
///
/// # Customization
///
/// How `FileServer` responds to specific requests can be customized, through
/// the use of [`Rewriter`]s. See [`Rewriter`] for more detailed documentation
/// on how to take full advantage of the customization of `FileServer`.
///
/// [`FileServer::from()`] and [`FileServer::new()`] automatically add some common
/// rewrites. They filter out dotfiles, redirect folder accesses to include a trailing
/// slash, and use `index.html` to respond to requests for a directory. If you want
/// to customize or replace these default rewrites, see [`FileServer::empty()`].
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

impl fmt::Debug for FileServer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileServer")
            // .field("root", &self.root)
            .field("rewrites", &DebugListRewrite(&self.rewrites))
            .field("rank", &self.rank)
            .finish()
    }
}

struct DebugListRewrite<'a>(&'a Vec<Arc<dyn Rewriter>>);

impl fmt::Debug for DebugListRewrite<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{} rewrites>", self.0.len())
    }
}

/// Trait used to implement [`FileServer`] customization.
///
/// Conceptually, a [`FileServer`] is a sequence of `Rewriter`s, which transform
/// a path from a request to a final response. [`FileServer`] add a set of default
/// `Rewriter`s, which filter out dotfiles, apply a root path, normalize directories,
/// and use `index.html`.
///
/// After running the chain of `Rewriter`s,
/// [`FileServer`] uses the final [`Option<FileResponse>`](FileResponse)
/// to respond to the request. If the response is `None`, a path that doesn't
/// exist or a directory path, [`FileServer`] will respond with a
/// [`Status::NotFound`](crate::http::Status::NotFound). Otherwise the [`FileServer`]
/// will respond with a redirect or the contents of the file specified.
///
/// [`FileServer`] provides several helper methods to add `Rewriter`s:
/// - [`FileServer::and_rewrite()`]
/// - [`FileServer::filter_file()`]
/// - [`FileServer::map_file()`]
pub trait Rewriter: Send + Sync + 'static {
    /// Alter the [`FileResponse`] as needed.
    fn rewrite<'p, 'h>(&self, path: Option<FileResponse<'p, 'h>>, req: &Request<'_>)
        -> Option<FileResponse<'p, 'h>>;
}

/// A Response from a [`FileServer`]
#[derive(Debug)]
#[non_exhaustive]
pub enum FileResponse<'p, 'h> {
    /// Return the contents of the specified file.
    File(File<'p, 'h>),
    /// Returns a Redirect to the specified path. This needs to be an absolute
    /// URI, so you should start with [`File.full_uri`](File) when constructing
    /// a redirect.
    Redirect(Redirect),
}

impl<'p, 'h> From<File<'p, 'h>> for FileResponse<'p, 'h> {
    fn from(value: File<'p, 'h>) -> Self {
        Self::File(value)
    }
}
impl<'p, 'h> From<File<'p, 'h>> for Option<FileResponse<'p, 'h>> {
    fn from(value: File<'p, 'h>) -> Self {
        Some(FileResponse::File(value))
    }
}

impl<'p, 'h> From<Redirect> for FileResponse<'p, 'h> {
    fn from(value: Redirect) -> Self {
        Self::Redirect(value)
    }
}
impl<'p, 'h> From<Redirect> for Option<FileResponse<'p, 'h>> {
    fn from(value: Redirect) -> Self {
        Some(FileResponse::Redirect(value))
    }
}

/// A File response from a [`FileServer`]
#[derive(Debug)]
pub struct File<'p, 'h> {
    /// The path to the file that [`FileServer`] will respond with.
    pub path: Cow<'p, Path>,
    /// A list of headers to be added to the generated response.
    pub headers: HeaderMap<'h>,
}

impl<'p, 'h> File<'p, 'h> {
    /// Add a header to this `File`.
    pub fn with_header<'n: 'h, H: Into<Header<'n>>>(mut self, header: H) -> Self {
        self.headers.add(header);
        self
    }

    /// Replace the path of this `File`.
    pub fn with_path(self, path: impl Into<Cow<'p, Path>>) -> Self {
        Self {
            path: path.into(),
            headers: self.headers,
        }
    }

    /// Replace the path of this `File`.
    pub fn map_path<R: Into<Cow<'p, Path>>>(self, f: impl FnOnce(Cow<'p, Path>) -> R) -> Self {
        Self {
            path: f(self.path).into(),
            headers: self.headers,
        }
    }

    // /// Convert this `File` into a Redirect, transforming the URI.
    // pub fn into_redirect(self, f: impl FnOnce(Origin<'static>) -> Origin<'static>)
    //     -> FileResponse<'p, 'h>
    // {
    //     FileResponse::Redirect(Redirect::permanent(f(self.full_uri.clone().into_owned())))
    // }

    async fn respond_to<'r>(self, req: &'r Request<'_>, data: Data<'r>) -> Outcome<'r>
        where 'h: 'r
    {
        /// Normalize paths to enable `file_root` to work properly
        fn strip_trailing_slash(p: &Path) -> &Path {
            let bytes = p.as_os_str().as_encoded_bytes();
            let bytes = bytes.strip_suffix(MAIN_SEPARATOR_STR.as_bytes()).unwrap_or(bytes);
            // SAFETY: Since we stripped a valid UTF-8 sequence (or left it unchanged),
            // this is still a valid OsStr.
            Path::new(unsafe { OsStr::from_encoded_bytes_unchecked(bytes) })
        }

        let path = strip_trailing_slash(self.path.as_ref());
        // Fun fact, on Linux attempting to open a directory works, it just errors
        // when you attempt to read it.
        if path.is_file() {
            NamedFile::open(path)
                .await
                .respond_to(req)
                .map(|mut r| {
                    for header in self.headers {
                        r.adjoin_raw_header(header.name.as_str().to_owned(), header.value);
                    }
                    r
                }).or_forward((data, Status::NotFound))
        } else {
            Outcome::forward(data, Status::NotFound)
        }
    }
}

impl<F: Send + Sync + 'static> Rewriter for F
    where F: for<'r, 'h> Fn(Option<FileResponse<'r, 'h>>, &Request<'_>)
        -> Option<FileResponse<'r, 'h>>
{
    fn rewrite<'p, 'h>(&self, path: Option<FileResponse<'p, 'h>>, req: &Request<'_>)
        -> Option<FileResponse<'p, 'h>>
    {
        self(path, req)
    }
}

/// Helper to implement [`FileServer::filter_file()`]
struct FilterFile<F>(F);
impl<F> Rewriter for FilterFile<F>
    where F: Fn(&File<'_, '_>, &Request<'_>) -> bool + Send + Sync + 'static
{
    fn rewrite<'p, 'h>(&self, path: Option<FileResponse<'p, 'h>>, req: &Request<'_>)
        -> Option<FileResponse<'p, 'h>>
    {
        match path {
            Some(FileResponse::File(file)) if !self.0(&file, req) => None,
            path => path,
        }
    }
}

/// Helper to implement [`FileServer::map_file()`]
struct MapFile<F>(F);
impl<F> Rewriter for MapFile<F>
    where F: for<'p, 'h> Fn(File<'p, 'h>, &Request<'_>)
        -> FileResponse<'p, 'h> + Send + Sync + 'static,
{
    fn rewrite<'p, 'h>(&self, path: Option<FileResponse<'p, 'h>>, req: &Request<'_>)
        -> Option<FileResponse<'p, 'h>>
    {
        match path {
            Some(FileResponse::File(file)) => Some(self.0(file, req)),
            path => path,
        }
    }
}

/// Helper trait to simplify standard rewrites
#[doc(hidden)]
pub trait FileMap:
    for<'p, 'h> Fn(File<'p, 'h>, &Request<'_>) -> FileResponse<'p, 'h> + Send + Sync + 'static
{}
impl<F> FileMap for F
    where F: for<'p, 'h> Fn(File<'p, 'h>, &Request<'_>)
        -> FileResponse<'p, 'h> + Send + Sync + 'static
{}
/// Helper trait to simplify standard rewrites
#[doc(hidden)]
pub trait FileFilter: Fn(&File<'_, '_>, &Request<'_>) -> bool + Send + Sync + 'static {}
impl<F> FileFilter for F
    where F: Fn(&File<'_, '_>, &Request<'_>) -> bool + Send + Sync + 'static
{}

/// Prepends the provided path, to serve files from a directory.
///
/// You can use [`relative!`] to make a path relative to the crate root, rather
/// than the runtime directory.
///
/// # Example
///
/// ```rust,no_run
/// # use rocket::fs::{FileServer, dir_root, relative};
/// # fn make_server() -> FileServer {
/// FileServer::empty()
///     .map_file(dir_root(relative!("static")))
/// # }
/// ```
///
/// # Panics
///
/// Panics if `path` does not exist. See [`file_root_permissive`] for a
/// non-panicing variant.
pub fn dir_root(path: impl AsRef<Path>) -> impl FileMap {
    let path = path.as_ref();
    if !path.is_dir() {
        let path = path.display();
        error!(%path, "FileServer path is not a directory.");
        warn!("Aborting early to prevent inevitable handler error.");
        panic!("invalid directory: refusing to continue");
    }
    let path = path.to_path_buf();
    move |f, _r| {
        FileResponse::File(f.map_path(|p| path.join(p)))
    }
}

/// Prepends the provided path, to serve a single static file.
///
/// # Example
///
/// ```rust,no_run
/// # use rocket::fs::{FileServer, file_root};
/// # fn make_server() -> FileServer {
/// FileServer::empty()
///     .map_file(file_root("static/index.html"))
/// # }
/// ```
///
/// # Panics
///
/// Panics if `path` does not exist. See [`file_root_permissive`] for a
/// non-panicing variant.
pub fn file_root(path: impl AsRef<Path>) -> impl FileMap {
    let path = path.as_ref();
    if !path.exists() {
        let path = path.display();
        error!(%path, "FileServer path does not exist.");
        warn!("Aborting early to prevent inevitable handler error.");
        panic!("invalid file: refusing to continue");
    }
    let path = path.to_path_buf();
    move |f, _r| {
        FileResponse::File(f.map_path(|p| path.join(p)))
    }
}

/// Prepends the provided path, without checking to ensure the path exists during
/// startup.
///
/// # Example
///
/// ```rust,no_run
/// # use rocket::fs::{FileServer, file_root_permissive};
/// # fn make_server() -> FileServer {
/// FileServer::empty()
///     .map_file(file_root_permissive("/tmp/rocket"))
/// # }
/// ```
pub fn file_root_permissive(path: impl AsRef<Path>) -> impl FileMap {
    let path = path.as_ref().to_path_buf();
    move |f, _r| {
        FileResponse::File(f.map_path(|p| path.join(p)))
    }
}

/// Filters out any path that contains a file or directory name starting with a
/// dot. If used after `dir_root`, this will also check the root path for dots, and
/// filter them.
///
/// # Example
///
/// ```rust,no_run
/// # use rocket::fs::{FileServer, filter_dotfiles, dir_root};
/// # fn make_server() -> FileServer {
/// FileServer::empty()
///     .filter_file(filter_dotfiles)
///     .map_file(dir_root("static"))
/// # }
/// ```
pub fn filter_dotfiles(file: &File<'_, '_>, _req: &Request<'_>) -> bool {
    !file.path.iter().any(|s| s.as_encoded_bytes().starts_with(b"."))
}

/// Normalize directory accesses to always include a trailing slash.
///
/// Should normally be used after `dir_root` (or another rewrite that adds
/// a root), since it needs the full path to check whether a path points to
/// a directory.
///
/// # Example
///
/// Appends a slash to any request for a directory without a trailing slash
/// ```rust,no_run
/// # use rocket::fs::{FileServer, normalize_dirs, dir_root};
/// # fn make_server() -> FileServer {
/// FileServer::empty()
///     .map_file(dir_root("static"))
///     .map_file(normalize_dirs)
/// # }
/// ```
pub fn normalize_dirs<'p, 'h>(file: File<'p, 'h>, req: &Request<'_>) -> FileResponse<'p, 'h> {
    if !req.uri().path().raw().ends_with('/') && file.path.is_dir() {
        FileResponse::Redirect(Redirect::permanent(
            // Known good path + '/' is a good path
            req.uri().clone().into_owned().map_path(|p| format!("{p}/")).unwrap()
        ))
    } else {
        FileResponse::File(file)
    }
}

/// Appends a file name to all directory accesses.
///
/// Must be used after `dir_root`, since it needs the full path to check whether it is
/// a directory.
///
/// # Example
///
/// Appends `index.html` to any directory access.
/// ```rust,no_run
/// # use rocket::fs::{FileServer, index, dir_root};
/// # fn make_server() -> FileServer {
/// FileServer::empty()
///     .map_file(dir_root("static"))
///     .map_file(index("index.html"))
/// # }
/// ```
pub fn index(index: &'static str) -> impl FileMap {
    move |f, _r| if f.path.is_dir() {
        FileResponse::File(f.map_path(|p| p.join(index)))
    } else {
        FileResponse::File(f)
    }
}

impl FileServer {
    /// The default rank use by `FileServer` routes.
    const DEFAULT_RANK: isize = 10;

    /// Constructs a new `FileServer`, with default rank, and no
    /// rewrites.
    ///
    /// See [`FileServer::empty_ranked()`].
    pub fn empty() -> Self {
        Self::empty_ranked(Self::DEFAULT_RANK)
    }

    /// Constructs a new `FileServer`, with specified rank, and no
    /// rewrites.
    ///
    /// # Example
    ///
    /// Replicate the output of [`FileServer::new()`].
    /// ```rust,no_run
    /// # use rocket::fs::{FileServer, filter_dotfiles, dir_root, normalize_dirs};
    /// # fn launch() -> FileServer {
    /// FileServer::empty_ranked(10)
    ///     .filter_file(filter_dotfiles)
    ///     .map_file(dir_root("static"))
    ///     .map_file(normalize_dirs)
    /// # }
    /// ```
    pub fn empty_ranked(rank: isize) -> Self {
        Self {
            rewrites: vec![],
            rank,
        }
    }

    /// Constructs a new `FileServer`, with the defualt rank of 10.
    ///
    /// See [`FileServer::new`].
    pub fn from<P: AsRef<Path>>(path: P) -> Self {
        Self::new(path, Self::DEFAULT_RANK)
    }

    /// Constructs a new `FileServer` that serves files from the file system
    /// `path`, with the specified rank.
    ///
    /// Adds a set of default rewrites:
    /// - [`filter_dotfiles`]: Hides all dotfiles.
    /// - [`dir_root(path)`](dir_root): Applies the root path.
    /// - [`normalize_dirs`]: Normalizes directories to have a trailing slash.
    /// - [`index("index.html")`](index): Appends `index.html` to directory requests.
    pub fn new<P: AsRef<Path>>(path: P, rank: isize) -> Self {
        Self::empty_ranked(rank)
            .filter_file(filter_dotfiles)
            .map_file(dir_root(path))
            .map_file(normalize_dirs)
            .map_file(index("index.html"))
    }

    /// Generic rewrite to transform one FileResponse to another.
    ///
    /// # Example
    ///
    /// Redirects all requests that have been filtered to the root of the `FileServer`.
    /// ```rust,no_run
    /// # use rocket::{fs::{FileServer, FileResponse}, response::Redirect,
    /// #     uri, Build, Rocket, Request};
    /// fn redir_missing<'p, 'h>(p: Option<FileResponse<'p, 'h>>, _req: &Request<'_>)
    ///     -> Option<FileResponse<'p, 'h>>
    /// {
    ///     match p {
    ///         None => Redirect::temporary(uri!("/")).into(),
    ///         p => p,
    ///     }
    /// }
    ///
    /// # fn launch() -> Rocket<Build> {
    /// rocket::build()
    ///     .mount("/", FileServer::from("static").and_rewrite(redir_missing))
    /// # }
    /// ```
    ///
    /// Note that `redir_missing` is not a closure in this example. Making it a closure
    /// causes compilation to fail with a lifetime error. It really shouldn't but it does.
    pub fn and_rewrite(mut self, f: impl Rewriter) -> Self {
        self.rewrites.push(Arc::new(f));
        self
    }

    /// Filter what files this `FileServer` will respond with
    ///
    /// # Example
    ///
    /// Filter out all paths with a filename of `hidden`.
    /// ```rust,no_run
    /// # use rocket::{fs::FileServer, response::Redirect, uri, Rocket, Build};
    /// # fn launch() -> Rocket<Build> {
    /// rocket::build()
    ///     .mount(
    ///         "/",
    ///         FileServer::from("static")
    ///            .filter_file(|f, _r| f.path.file_name() != Some("hidden".as_ref()))
    ///     )
    /// # }
    /// ```
    pub fn filter_file<F>(self, f: F) -> Self
        where F: Fn(&File<'_, '_>, &Request<'_>) -> bool + Send + Sync + 'static
    {
        self.and_rewrite(FilterFile(f))
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
    ///             .map_file(|f, _r| f.map_path(|p| p.join("hidden")).into())
    ///     )
    /// # }
    /// ```
    pub fn map_file<F>(self, f: F) -> Self
        where F: for<'r, 'h> Fn(File<'r, 'h>, &Request<'_>)
            -> FileResponse<'r, 'h> + Send + Sync + 'static
    {
        self.and_rewrite(MapFile(f))
    }
}

impl From<FileServer> for Vec<Route> {
    fn from(server: FileServer) -> Self {
        // let source = figment::Source::File(server.root.clone());
        let mut route = Route::ranked(server.rank, Method::Get, "/<path..>", server);
        // I'd like to provide a more descriptive name, but we can't get more
        // information out of `dyn Rewriter`
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
        let mut response = path.as_ref().map(|p| FileResponse::File(File {
            path: Cow::Borrowed(p),
            headers: HeaderMap::new(),
        }));

        for rewrite in &self.rewrites {
            response = rewrite.rewrite(response, req);
        }

        match response {
            Some(FileResponse::File(file)) => file.respond_to(req, data).await,
            Some(FileResponse::Redirect(r)) => {
                r.respond_to(req)
                    .or_forward((data, Status::InternalServerError))
            },
            None => Outcome::forward(data, Status::NotFound),
        }
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
