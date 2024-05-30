use core::fmt;
use std::any::{type_name, Any, TypeId};
use std::borrow::Cow;
use std::os::unix::ffi::OsStrExt;
use std::path::{PathBuf, Path};
use std::sync::Arc;

use crate::fs::NamedFile;
use crate::{Data, Request, outcome::IntoOutcome};
use crate::http::{Method, HeaderMap, Header, uri::{Segments, Reference}, Status};
use crate::route::{Route, Handler, Outcome};
use crate::response::{Redirect, Responder};

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
    // root: PathBuf,
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
        f.debug_list().entries(self.0.iter().map(|r| r.name())).finish()
    }
}

pub trait Rewriter: Send + Sync + Any {
    /// Modify RewritablePath as needed.
    fn rewrite<'a>(&self, path: Option<FileResponse<'a>>) -> Option<FileResponse<'a>>;
    /// provides type name for debug printing
    fn name(&self) -> &'static str { type_name::<Self>() }
}

#[derive(Debug)]
pub enum FileResponse<'a> {
    /// Status: Ok
    File(File<'a>),
    /// Status: Redirect
    Redirect(Redirect),
}

#[derive(Debug)]
pub struct File<'a> {
    pub path: Cow<'a, Path>,
    pub headers: HeaderMap<'a>,
}

impl<'a> File<'a> {
    pub fn with_header<'h: 'a, H: Into<Header<'h>>>(mut self, header: H) -> Self {
        self.headers.add(header);
        self
    }

    pub fn with_path(self, path: impl Into<Cow<'a, Path>>) -> Self {
        Self {
            path: path.into(),
            headers: self.headers,
        }
    }

    pub fn modify_path(self, f: impl FnOnce(&mut PathBuf)) -> Self {
        let mut path = self.path.into_owned();
        f(&mut path);
        Self {
            path: path.into(),
            headers: self.headers,
        }
    }
}



impl<F: Send + Sync + Any> Rewriter for F
    where F: for<'r> Fn(Option<FileResponse<'r>>) -> Option<FileResponse<'r>>
{
    fn rewrite<'a>(&self, path: Option<FileResponse<'a>>) -> Option<FileResponse<'a>> {
        self(path)
    }

    fn name(&self) -> &'static str {
        "Custom Rewrite"
    }
}

pub struct FilterFile<F>(F);
impl<F: Fn(&File<'_>) -> bool + Send + Sync + Any> Rewriter for FilterFile<F> {
    fn rewrite<'a>(&self, path: Option<FileResponse<'a>>) -> Option<FileResponse<'a>> {
        match path {
            Some(FileResponse::File(file)) if !self.0(&file) => None,
            path => path,
        }
    }

    fn name(&self) -> &'static str {
        "Custom file filter"
    }
}

pub struct FilterRedirect<F>(F, Reference<'static>);
impl<F: Fn(&File<'_>) -> bool + Send + Sync + Any> Rewriter for FilterRedirect<F> {
    fn rewrite<'a>(&self, path: Option<FileResponse<'a>>) -> Option<FileResponse<'a>> {
        match path {
            Some(FileResponse::File(file)) if !self.0(&file) =>
                Some(FileResponse::Redirect(Redirect::permanent(self.1.clone()))),
            path => path,
        }
    }

    fn name(&self) -> &'static str {
        "Custom file filter with redirect"
    }
}

pub struct MapFile<F>(F);
impl<F: for<'r> Fn(File<'r>) -> File<'r> + Send + Sync + Any> Rewriter for MapFile<F> {
    fn rewrite<'a>(&self, path: Option<FileResponse<'a>>) -> Option<FileResponse<'a>> {
        match path {
            Some(FileResponse::File(file)) => Some(FileResponse::File(self.0(file))),
            path => path,
        }
    }

    fn name(&self) -> &'static str {
        "Custom file map"
    }
}

pub struct Root(PathBuf);
impl Rewriter for Root {
    fn rewrite<'a>(&self, path: Option<FileResponse<'a>>) -> Option<FileResponse<'a>> {
        match path {
            Some(FileResponse::File(file)) => Some(FileResponse::File(File {
                path: self.0.join(file.path).into(),
                headers: file.headers,
            })),
            path => path,
        }
    }
}
impl Rewriter for MapFile<Root> {
    fn rewrite<'a>(&self, path: Option<FileResponse<'a>>) -> Option<FileResponse<'a>> {
        self.0.rewrite(path)
    }
}

pub struct BlockDotfiles;
impl Rewriter for BlockDotfiles {
    fn rewrite<'a>(&self, path: Option<FileResponse<'a>>) -> Option<FileResponse<'a>> {
        match path {
            Some(FileResponse::File(file)) if file.path.iter()
                .any(|seg| seg.as_bytes().starts_with(b".")) => None,
            path => path,
        }
    }
}
impl Rewriter for FilterFile<BlockDotfiles> {
    fn rewrite<'a>(&self, path: Option<FileResponse<'a>>) -> Option<FileResponse<'a>> {
        self.0.rewrite(path)
    }
}

/// Normalize directory accesses to always include a trailing slash.
///
/// Must be used before `Root`, otherwise it will redirect to the full
/// path, including the root directory.
pub struct NormalizeDirs;
impl Rewriter for NormalizeDirs {
    fn rewrite<'a>(&self, path: Option<FileResponse<'a>>) -> Option<FileResponse<'a>> {
        match path {
            // This 
            Some(FileResponse::File(file)) if
                    !file.path.as_os_str().as_encoded_bytes().ends_with(b"/")
                    && file.path.is_dir()
                => Some(FileResponse::Redirect(
                    Redirect::permanent(format!("{}/", file.path.display()))
                )),
            path => path,
        }
    }
}
impl Rewriter for MapFile<NormalizeDirs> {
    fn rewrite<'a>(&self, path: Option<FileResponse<'a>>) -> Option<FileResponse<'a>> {
        self.0.rewrite(path)
    }
}

/// Appends a file name to all directory accesses. `index.html` by default
pub struct Index(pub &'static str);
impl Rewriter for Index {
    fn rewrite<'a>(&self, path: Option<FileResponse<'a>>) -> Option<FileResponse<'a>> {
        match path {
            Some(FileResponse::File(file)) if file.path.is_dir()
                => Some(FileResponse::File(file.modify_path(|p| p.push(self.0)))),
            path => path,
        }
    }
}
impl Rewriter for MapFile<Index> {
    fn rewrite<'a>(&self, path: Option<FileResponse<'a>>) -> Option<FileResponse<'a>> {
        self.0.rewrite(path)
    }
}
impl Default for Index {
    fn default() -> Self {
        Self("index.html")
    }
}

impl FileServer {
    /// The default rank use by `FileServer` routes.
    const DEFAULT_RANK: isize = 10;

    /// Constructs a new `FileServer`, with default rank, and no
    /// rewrites.
    pub fn empty() -> Self {
        Self {
            rewrites: vec![],
            rank: Self::DEFAULT_RANK,
        }
    }

    /// Constructs a new `FileServer`, with the defualt rank of 10.
    ///
    /// See: [`FileServer::new`] for more details
    pub fn from<P: AsRef<Path>>(path: P) -> Self {
        Self::new(path, Self::DEFAULT_RANK)
    }

    /// Constructs a new `FileServer` that serves files from the file system
    /// `path`, with the specified rank.
    /// 
    /// Adds a set of default rewrites:
    /// - [`BlockDotfiles`]: Hides all dotfiles
    /// - [`NormalizeDirs`]: Normalizes directories to have a trailing slash
    /// - [`Root(path)`](Root): Applies the root path
    pub fn new<P: AsRef<Path>>(path: P, rank: isize) -> Self {
        use crate::yansi::Paint;

        let path = path.as_ref();
        if !path.exists() {
            let path = path.display();
            error!("FileServer path '{}' does not exist", path.primary());
            // warn_!("Aborting early to prevent inevitable handler error.");
        }
        FileServer { rewrites: vec![
            Arc::new(BlockDotfiles),
            Arc::new(NormalizeDirs),
            Arc::new(Root(path.into()))
        ], rank }
    }

    /// Removes all rewrites of a specific type. This is primarily useful for removing
    /// rewrites added by default, such as `BlockDotfiles`, `NormalizeDirs`, and `Root`
    pub fn remove_rewrites<R: Rewriter + Any>(mut self) -> Self {
        self.rewrites.retain(|r| { 
            <_ as AsRef<dyn Rewriter>>::as_ref(&r).type_id() != TypeId::of::<R>()
        });
        self
    }

    /// Generic rewrite to transform one FileResponse to another
    pub fn and_rewrite(mut self, f: impl Rewriter + 'static) -> Self {
        self.rewrites.push(Arc::new(f));
        self
    }

    /// Configure this `FileServer` instance to allow serving dotfiles.
    pub fn allow_dotfiles(self) -> Self {
        self.remove_rewrites::<BlockDotfiles>()
    }

    /// Filter what files this `FileServer` will respond with
    pub fn filter_file<F>(self, f: F) -> Self
        // where F: Fn(&File<'_>) -> bool + Send + Sync + Any
        where FilterFile<F>: Rewriter
    {
        self.and_rewrite(FilterFile(f))
    }

    /// Filter what files this `FileServer` will respond with
    pub fn filter_file_or_redirect<F>(self, f: F, to: Reference<'static>) -> Self
        where F: Fn(&File<'_>) -> bool + Send + Sync + Any
    {
        self.and_rewrite(FilterRedirect(f, to))
    }

    /// Transform files before responding
    pub fn map_file<F>(self, f: F) -> Self
        // where F: for<'r> Fn(File<'r>) -> File<'r> + Send + Sync + Any
        where MapFile<F>: Rewriter
    {
        self.and_rewrite(MapFile(f))
    }
}

impl From<FileServer> for Vec<Route> {
    fn from(server: FileServer) -> Self {
        // let source = figment::Source::File(server.root.clone());
        let mut route = Route::ranked(server.rank, Method::Get, "/<path..>", server);
        route.name = Some(format!("FileServer").into());
        vec![route]
    }
}

#[crate::async_trait]
impl Handler for FileServer {
    async fn handle<'r>(&self, req: &'r Request<'_>, data: Data<'r>) -> Outcome<'r> {
        use crate::http::uri::fmt::Path as UriPath;
        let mut response = req.segments::<Segments<'_, UriPath>>(0..).ok()
            .and_then(|segments| segments.to_path_buf(true).ok())
            .map(|path| FileResponse::File(File { path: path.into(), headers: HeaderMap::new() }));

        for rewrite in &self.rewrites {
            response = rewrite.rewrite(response);
        }
        match response {
            Some(FileResponse::File(File { path, headers })) => {
                NamedFile::open(path).await.respond_to(req).map(|mut r| {
                    for header in headers {
                        r.adjoin_raw_header(header.name.as_str().to_owned(), header.value);
                    }
                    r
                }).or_forward((data, Status::NotFound))
            },
            Some(FileResponse::Redirect(r)) => {
                r.respond_to(req).or_forward((data, Status::InternalServerError))
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
