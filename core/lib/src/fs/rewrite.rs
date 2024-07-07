use std::borrow::Cow;
use std::path::{Path, PathBuf};

use crate::Request;
use crate::http::{ext::IntoOwned, HeaderMap};
use crate::response::Redirect;

/// A file server [`Rewrite`] rewriter.
///
/// A [`FileServer`] is a sequence of [`Rewriter`]s which transform the incoming
/// request path into a [`Rewrite`] or `None`. The first rewriter is called with
/// the request path as a [`Rewrite::File`]. Each `Rewriter` thereafter is
/// called in-turn with the previously returned [`Rewrite`], and the value
/// returned from the last `Rewriter` is used to respond to the request. If the
/// final rewrite is `None` or a nonexistent path or a directory, [`FileServer`]
/// responds with [`Status::NotFound`]. Otherwise it responds with the file
/// contents, if [`Rewrite::File`] is specified, or a redirect, if
/// [`Rewrite::Redirect`] is specified.
///
/// [`FileServer`]: super::FileServer
/// [`Status::NotFound`]: crate::http::Status::NotFound
pub trait Rewriter: Send + Sync + 'static {
    /// Alter the [`Rewrite`] as needed.
    fn rewrite<'r>(&self, opt: Option<Rewrite<'r>>, req: &'r Request<'_>) -> Option<Rewrite<'r>>;
}

/// A Response from a [`FileServer`](super::FileServer)
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Rewrite<'r> {
    /// Return the contents of the specified file.
    File(File<'r>),
    /// Returns a Redirect.
    Redirect(Redirect),
}

/// A File response from a [`FileServer`](super::FileServer) and a rewriter.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct File<'r> {
    /// The path to the file that [`FileServer`](super::FileServer) will respond with.
    pub path: Cow<'r, Path>,
    /// A list of headers to be added to the generated response.
    pub headers: HeaderMap<'r>,
}

impl<'r> File<'r> {
    /// A new `File`, with not additional headers.
    pub fn new(path: impl Into<Cow<'r, Path>>) -> Self {
        Self { path: path.into(), headers: HeaderMap::new() }
    }

    /// A new `File`, with not additional headers.
    ///
    /// # Panics
    ///
    /// Panics if the `path` does not exist.
    pub fn checked<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref();
        if !path.exists() {
            let path = path.display();
            error!(%path, "FileServer path does not exist.\n\
                Panicking to prevent inevitable handler error.");
            panic!("missing file {}: refusing to continue", path);
        }

        Self::new(path.to_path_buf())
    }

    /// Replace the path in `self` with the result of applying `f` to the path.
    pub fn map_path<F, P>(self, f: F) -> Self
        where F: FnOnce(Cow<'r, Path>) -> P, P: Into<Cow<'r, Path>>,
    {
        Self {
            path: f(self.path).into(),
            headers: self.headers,
        }
    }

    /// Returns `true` if the file is a dotfile. A dotfile is a file whose
    /// name or any directory in it's path start with a period (`.`) and is
    /// considered hidden.
    ///
    /// # Windows Note
    ///
    /// This does *not* check the file metadata on any platform, so hidden files
    /// on Windows will not be detected by this method.
    pub fn is_hidden(&self) -> bool {
        self.path.iter().any(|s| s.as_encoded_bytes().starts_with(b"."))
    }

    /// Returns `true` if the file is not hidden. This is the inverse of
    /// [`File::is_hidden()`].
    pub fn is_visible(&self) -> bool {
        !self.is_hidden()
    }
}

/// Prefixes all paths with a given path.
///
/// # Example
///
/// ```rust,no_run
/// use rocket::fs::FileServer;
/// use rocket::fs::rewrite::Prefix;
///
/// FileServer::identity()
///    .filter(|f, _| f.is_visible())
///    .rewrite(Prefix::checked("static"));
/// ```
pub struct Prefix(PathBuf);

impl Prefix {
    /// Panics if `path` does not exist.
    pub fn checked<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref();
        if !path.is_dir() {
            let path = path.display();
            error!(%path, "FileServer path is not a directory.");
            warn!("Aborting early to prevent inevitable handler error.");
            panic!("invalid directory: refusing to continue");
        }

        Self(path.to_path_buf())
    }

    /// Creates a new `Prefix` from a path.
    pub fn unchecked<P: AsRef<Path>>(path: P) -> Self {
        Self(path.as_ref().to_path_buf())
    }
}

impl Rewriter for Prefix {
    fn rewrite<'r>(&self, opt: Option<Rewrite<'r>>, _: &Request<'_>) -> Option<Rewrite<'r>> {
        opt.map(|r| match r {
            Rewrite::File(f) => Rewrite::File(f.map_path(|p| self.0.join(p))),
            Rewrite::Redirect(r) => Rewrite::Redirect(r),
        })
    }
}

impl Rewriter for PathBuf {
    fn rewrite<'r>(&self, _: Option<Rewrite<'r>>, _: &Request<'_>) -> Option<Rewrite<'r>> {
        Some(Rewrite::File(File::new(self.clone())))
    }
}

/// Normalize directories to always include a trailing slash by redirecting
/// (with a 302 temporary redirect) requests for directories without a trailing
/// slash to the same path with a trailing slash.
///
/// # Example
///
/// ```rust,no_run
/// use rocket::fs::FileServer;
/// use rocket::fs::rewrite::{Prefix, TrailingDirs};
///
/// FileServer::identity()
///     .filter(|f, _| f.is_visible())
///     .rewrite(TrailingDirs);
/// ```
pub struct TrailingDirs;

impl Rewriter for TrailingDirs {
    fn rewrite<'r>(&self, opt: Option<Rewrite<'r>>, req: &Request<'_>) -> Option<Rewrite<'r>> {
        if let Some(Rewrite::File(f)) = &opt {
            if !req.uri().path().ends_with('/') && f.path.is_dir() {
                let uri = req.uri().clone().into_owned();
                let uri = uri.map_path(|p| format!("{p}/")).unwrap();
                return Some(Rewrite::Redirect(Redirect::temporary(uri)));
            }
        }

        opt
    }
}

/// Rewrite a directory to a file inside of that directory.
///
/// # Example
///
/// Rewrites all directory requests to `directory/index.html`.
///
/// ```rust,no_run
/// use rocket::fs::FileServer;
/// use rocket::fs::rewrite::DirIndex;
///
/// FileServer::without_index("static")
///     .rewrite(DirIndex::if_exists("index.htm"))
///     .rewrite(DirIndex::unconditional("index.html"));
/// ```
pub struct DirIndex {
    path: PathBuf,
    check: bool,
}

impl DirIndex {
    /// Appends `path` to every request for a directory.
    pub fn unconditional(path: impl AsRef<Path>) -> Self {
        Self { path: path.as_ref().to_path_buf(), check: false }
    }

    /// Only appends `path` to a request for a directory if the file exists.
    pub fn if_exists(path: impl AsRef<Path>) -> Self {
        Self { path: path.as_ref().to_path_buf(), check: true }
    }
}

impl Rewriter for DirIndex {
    fn rewrite<'r>(&self, opt: Option<Rewrite<'r>>, _: &Request<'_>) -> Option<Rewrite<'r>> {
        match opt? {
            Rewrite::File(f) if f.path.is_dir() => {
                let candidate = f.path.join(&self.path);
                if self.check && !candidate.is_file() {
                    return Some(Rewrite::File(f));
                }

                Some(Rewrite::File(f.map_path(|_| candidate)))
            }
            r => Some(r),
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
