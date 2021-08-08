#![allow(dead_code)]

use std::io;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};


use crate::{
    http::{
        ContentType,
        DateTimeOffset,
        EntityTagHeaderValue
    },
    request::Request,
    response::{self, Responder},
    tokio::{
        fs::File,
        io::AsyncSeek
    }
};

mod static_file_context;

pub use static_file_context::*;
use std::io::SeekFrom;

/// A [`Responder`] that sends file data with a Content-Type based on its
/// file extension.
///
/// # Example
///
/// A simple static file server mimicking [`FileServer`]:
///
/// ```rust
/// # use rocket::get;
/// use std::path::{PathBuf, Path};
///
/// use rocket::fs::{StaticFile, relative};
///
/// #[get("/file/<path..>")]
/// pub async fn second(path: PathBuf) -> Option<StaticFile> {
///     let mut path = Path::new(relative!("static")).join(path);
///     if path.is_dir() {
///         path.push("index.html");
///     }
///
///     StaticFile::open(path).await.ok()
/// }
/// ```
///
/// Always prefer to use [`FileServer`] which has more functionality and a
/// pithier API.
///
/// [`FileServer`]: crate::fs::FileServer
#[derive(Debug)]
pub struct StaticFile{
    path: PathBuf,
    file: File,
    content_type: Option<ContentType>,
    len: u64,
    last_modified: DateTimeOffset,
    etag: EntityTagHeaderValue,
}

impl StaticFile {
    /// Attempts to open a file in read-only mode.
    ///
    /// # Errors
    ///
    /// This function will return an error if path does not already exist. Other
    /// errors may also be returned according to
    /// [`OpenOptions::open()`](std::fs::OpenOptions::open()).
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rocket::get;
    /// use rocket::fs::StaticFile;
    ///
    /// #[get("/")]
    /// async fn index() -> Option<StaticFile> {
    ///     StaticFile::open("index.html").await.ok()
    /// }
    /// ```
    pub async fn open<P: AsRef<Path>>(path: P) -> io::Result<StaticFile> {
        // FIXME: Grab the file size here and prohibit `seek`ing later (or else
        // the file's effective size may change), to save on the cost of doing
        // all of those `seek`s to determine the file size. But, what happens if
        // the file gets changed between now and then?
        let path = path.as_ref();
        let file = File::open(path).await?;
        let metadata = file.metadata().await?;
        let len = metadata.len();
        let modified: DateTimeOffset = metadata.modified().unwrap().into();
        let etag_hash = modified.timestamp_millis() ^ len as i64;
        let etag_hash = format!("{:x}", etag_hash);
        let content_type = path.extension()
            .map(|ext| ContentType::from_extension(&ext.to_string_lossy()))
            .unwrap_or_default();

        Ok(StaticFile {
            path: path.to_path_buf(),
            file,
            content_type,
            len,
            etag: etag_hash.into(),
            last_modified: modified,
        })
    }

    /// Retrieve the underlying `File`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::fs::StaticFile;
    ///
    /// # async fn f() -> std::io::Result<()> {
    /// let named_file = StaticFile::open("index.html").await?;
    /// let file = named_file.file();
    /// # Ok(())
    /// # }
    /// ```
    #[inline(always)]
    pub fn file(&self) -> &File {
        &self.file
    }

    /// Retrieve a mutable borrow to the underlying `File`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::fs::StaticFile;
    ///
    /// # async fn f() -> std::io::Result<()> {
    /// let mut named_file = StaticFile::open("index.html").await?;
    /// let file = named_file.file_mut();
    /// # Ok(())
    /// # }
    /// ```
    #[inline(always)]
    pub fn file_mut(&mut self) -> &mut File {
        &mut self.file
    }

    /// Take the underlying `File`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::fs::StaticFile;
    ///
    /// # async fn f() -> std::io::Result<()> {
    /// let named_file = StaticFile::open("index.html").await?;
    /// let file = named_file.take_file();
    /// # Ok(())
    /// # }
    /// ```
    #[inline(always)]
    pub fn take_file(self) -> File {
        self.file
    }

    /// Retrieve the path of this file.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rocket::fs::StaticFile;
    ///
    /// # async fn demo_path() -> std::io::Result<()> {
    /// let file = StaticFile::open("foo.txt").await?;
    /// assert_eq!(file.path().as_os_str(), "foo.txt");
    /// # Ok(())
    /// # }
    /// ```
    #[inline(always)]
    pub fn path(&self) -> &Path {
        self.path.as_path()
    }
}

/// Streams the named file to the client. Sets or overrides the Content-Type in
/// the response according to the file's extension if the extension is
/// recognized. See [`ContentType::from_extension()`] for more information. If
/// you would like to stream a file with a different Content-Type than that
/// implied by its extension, use a [`File`] directly.
impl<'r> Responder<'r, 'static> for StaticFile {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        let mut static_ctx = StaticFileContext::from(
            (self.len, self.content_type, self.etag, self.last_modified)
        );
        static_ctx.comprehend_request_headers(req);
        let file = self.file;
        static_ctx.proceed(req, |(builder, pos ,len)| {
            let mut file = file;
            if pos > 0 {
                std::pin::Pin::new(&mut file)
                    .start_seek(SeekFrom::Start(pos))
                    .unwrap();
            }
            builder.sized_body(len as usize, file);
        })
    }
}

impl Deref for StaticFile {
    type Target = File;

    fn deref(&self) -> &File {
        &self.file
    }
}

impl DerefMut for StaticFile {
    fn deref_mut(&mut self) -> &mut File {
        &mut self.file
    }
}
