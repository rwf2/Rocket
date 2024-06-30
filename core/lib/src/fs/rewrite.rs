use std::borrow::Cow;
use std::path::Path;

use crate::{Request, Response};
use crate::http::{HeaderMap, ContentType};
use crate::http::ext::IntoOwned;
use crate::response::{self, Redirect, Responder};

/// Trait used to implement [`FileServer`] customization.
///
/// Conceptually, a [`FileServer`] is a sequence of `Rewriter`s, which transform
/// a path from a request to a final response. [`FileServer`] add a set of default
/// `Rewriter`s, which filter out dotfiles, apply a root path, normalize directories,
/// and use `index.html`.
///
/// After running the chain of `Rewriter`s,
/// [`FileServer`] uses the final [`Option<Rewrite>`](Rewrite)
/// to respond to the request. If the response is `None`, a path that doesn't
/// exist or a directory path, [`FileServer`] will respond with a
/// [`Status::NotFound`](crate::http::Status::NotFound). Otherwise the [`FileServer`]
/// will respond with a redirect or the contents of the file specified.
///
/// [`FileServer`] provides several helper methods to add `Rewriter`s:
/// - [`FileServer::rewrite()`]
/// - [`FileServer::filter()`]
/// - [`FileServer::map()`]
pub trait Rewriter: Send + Sync + 'static {
    /// Alter the [`Rewrite`] as needed.
    fn rewrite<'r>(&self, file: Option<Rewrite<'r>>, req: &'r Request<'_>) -> Option<Rewrite<'r>>;
}

/// A Response from a [`FileServer`]
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Rewrite<'r> {
    /// Return the contents of the specified file.
    File(File<'r>),
    /// Returns a Redirect.
    Redirect(Redirect),
}

/// A File response from a [`FileServer`]
#[derive(Debug, Clone)]
pub struct File<'r> {
    /// The path to the file that [`FileServer`] will respond with.
    pub path: Cow<'r, Path>,
    /// A list of headers to be added to the generated response.
    pub headers: HeaderMap<'r>,
}

impl<'r> Rewrite<'r> {
    pub fn file(&self) -> Option<&File<'r>> {
        match self {
            Rewrite::File(f) => Some(f),
            _ => None,
        }
    }

    pub fn redirect(&self) -> Option<&Redirect> {
        match self {
            Rewrite::Redirect(r) => Some(r),
            _ => None,
        }
    }
}

impl<'r> From<File<'r>> for Rewrite<'r> {
    fn from(value: File<'r>) -> Self {
        Self::File(value)
    }
}

impl<'r> From<Redirect> for Rewrite<'r> {
    fn from(value: Redirect) -> Self {
        Self::Redirect(value)
    }
}

impl<'r> File<'r> {
    pub fn new(path: impl Into<Cow<'r, Path>>) -> Self {
        Self { path: path.into(), headers: HeaderMap::new() }
    }

    pub(crate) async fn open(self) -> std::io::Result<NamedFile<'r>> {
        let file = tokio::fs::File::open(&self.path).await?;
        let metadata = file.metadata().await?;
        if metadata.is_dir() {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "is a directory"));
        }

        Ok(NamedFile {
            file,
            len: metadata.len(),
            path: self.path,
            headers: self.headers,
        })
    }

    /// Replace the path of this `File`.
    pub fn map_path<F, P>(self, f: F) -> Self
        where F: FnOnce(Cow<'r, Path>) -> P,
              P: Into<Cow<'r, Path>>,
    {
        Self {
            path: f(self.path).into(),
            headers: self.headers,
        }
    }
}

impl<F: Send + Sync + 'static> Rewriter for F
    where F: for<'r> Fn(Option<Rewrite<'r>>, &Request<'_>) -> Option<Rewrite<'r>>
{
    fn rewrite<'r>(&self, f: Option<Rewrite<'r>>, r: &Request<'_>) -> Option<Rewrite<'r>> {
        self(f, r)
    }
}

impl Rewriter for Rewrite<'static> {
    fn rewrite<'r>(&self, _: Option<Rewrite<'r>>, _: &Request<'_>) -> Option<Rewrite<'r>> {
        Some(self.clone())
    }
}

impl Rewriter for File<'static> {
    fn rewrite<'r>(&self, _: Option<Rewrite<'r>>, _: &Request<'_>) -> Option<Rewrite<'r>> {
        Some(Rewrite::File(self.clone()))
    }
}

impl Rewriter for Redirect {
    fn rewrite<'r>(&self, _: Option<Rewrite<'r>>, _: &Request<'_>) -> Option<Rewrite<'r>> {
        Some(Rewrite::Redirect(self.clone()))
    }
}

/// Helper trait to simplify standard rewrites
#[doc(hidden)]
pub trait FileMap: for<'r> Fn(File<'r>, &Request<'_>) -> Rewrite<'r> + Send + Sync + 'static {}
impl<F> FileMap for F
    where F: for<'r> Fn(File<'r>, &Request<'_>) -> Rewrite<'r> + Send + Sync + 'static {}

/// Helper trait to simplify standard rewrites
#[doc(hidden)]
pub trait FileFilter: Fn(&File<'_>, &Request<'_>) -> bool + Send + Sync + 'static {}
impl<F> FileFilter for F
    where F: Fn(&File<'_>, &Request<'_>) -> bool + Send + Sync + 'static {}

/// Prepends the provided path, to serve files from a directory.
///
/// You can use [`relative!`] to make a path relative to the crate root, rather
/// than the runtime directory.
///
/// # Example
///
/// ```rust,no_run
/// # use rocket::fs::{FileServer, prefix, relative};
/// # fn make_server() -> FileServer {
/// FileServer::empty()
///     .map(prefix(relative!("static")))
/// # }
/// ```
///
/// # Panics
///
/// Panics if `path` does not exist. See [`file_root_permissive`] for a
/// non-panicing variant.
pub fn prefix(path: impl AsRef<Path>) -> impl FileMap {
    let path = path.as_ref();
    if !path.is_dir() {
        let path = path.display();
        error!(%path, "FileServer path is not a directory.");
        warn!("Aborting early to prevent inevitable handler error.");
        panic!("invalid directory: refusing to continue");
    }

    let path = path.to_path_buf();
    move |f, _r| {
        Rewrite::File(f.map_path(|p| path.join(p)))
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
///     .map(file_root("static/index.html"))
/// # }
/// ```
///
/// # Panics
///
/// Panics if `path` does not exist. See [`file_root_permissive`] for a
/// non-panicing variant.
pub fn file_root(path: impl AsRef<Path>) -> impl Rewriter {
    let path = path.as_ref();
    if !path.exists() {
        let path = path.display();
        error!(%path, "FileServer path does not exist.");
        warn!("Aborting early to prevent inevitable handler error.");
        panic!("invalid file: refusing to continue");
    }

    Rewrite::File(File::new(path.to_path_buf()))
}

/// Rewrites the entire path with `path`. Does not check if `path` exists.
///
/// # Example
///
/// ```rust,no_run
/// # use rocket::fs::{FileServer, file_root_permissive};
/// # fn make_server() -> FileServer {
/// FileServer::empty()
///     .map(file_root_permissive("/tmp/rocket"))
/// # }
/// ```
pub fn file_root_permissive(path: impl AsRef<Path>) -> impl Rewriter {
    let path = path.as_ref().to_path_buf();
    Rewrite::File(File::new(path))
}

/// Filters out any path that contains a file or directory name starting with a
/// dot. If used after `prefix`, this will also check the root path for dots, and
/// filter them.
///
/// # Example
///
/// ```rust,no_run
/// # use rocket::fs::{FileServer, filter_dotfiles, prefix};
/// # fn make_server() -> FileServer {
/// FileServer::empty()
///     .filter(filter_dotfiles)
///     .map(prefix("static"))
/// # }
/// ```
pub fn filter_dotfiles(file: &File<'_>, _req: &Request<'_>) -> bool {
    !file.path.iter().any(|s| s.as_encoded_bytes().starts_with(b"."))
}

/// Normalize directory accesses to always include a trailing slash.
///
/// # Example
///
/// Appends a slash to any request for a directory without a trailing slash
/// ```rust,no_run
/// # use rocket::fs::{FileServer, normalize_dirs, prefix};
/// # fn make_server() -> FileServer {
/// FileServer::empty()
///     .map(prefix("static"))
///     .map(normalize_dirs)
/// # }
/// ```
pub fn normalize_dirs<'r>(file: File<'r>, req: &Request<'_>) -> Rewrite<'r> {
    if !req.uri().path().ends_with('/') && file.path.is_dir() {
        // Known good path + '/' is a good path.
        let uri = req.uri().clone().into_owned();
        Rewrite::Redirect(Redirect::temporary(uri.map_path(|p| format!("{p}/")).unwrap()))
    } else {
        Rewrite::File(file)
    }
}

/// Unconditionally rewrite a directory to a file named `index` inside of that
/// directory.
///
/// # Example
///
/// Rewrites all directory requests to `directory/index.html`.
///
/// ```rust,no_run
/// # use rocket::fs::{FileServer, index, prefix};
/// # fn make_server() -> FileServer {
/// FileServer::empty()
///     .map(prefix("static"))
///     .map(index("index.html"))
/// # }
/// ```
pub fn index(index: &'static str) -> impl FileMap {
    move |f, _r| if f.path.is_dir() {
        Rewrite::File(f.map_path(|p| p.join(index)))
    } else {
        Rewrite::File(f)
    }
}

/// Rewrite a directory to a file named `index` inside of that directory if that
/// file exists. Otherwise, leave the rewrite unchanged.
pub fn try_index(index: &'static str) -> impl FileMap {
    move |f, _r| if f.path.is_dir() {
        let original_path = f.path.clone();
        let index = original_path.join(index);
        if index.is_file() {
            Rewrite::File(f.map_path(|_| index))
        } else {
            Rewrite::File(f)
        }
    } else {
        Rewrite::File(f)
    }
}

pub(crate) struct NamedFile<'r> {
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
