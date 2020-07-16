use std::{fs, io};
use std::path::{Path, PathBuf};
use std::ops::{Deref, DerefMut};

use tokio::fs::File;

use crate::request::Request;
use crate::response::{self, Responder};
use crate::http::ContentType;
use std::time::SystemTime;
use time::{PrimitiveDateTime, OffsetDateTime};

// The spec for this header/format is described here
// https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Last-Modified#Syntax
// Last-Modified: <day-name>, <day> <month> <year> <hour>:<minute>:<second> GMT
// Example: Last-Modified: Sat, 03 Oct 2015 21:28:00 GMT

// Format codes can be found here. They are not compatible with libc strftime.
// https://docs.rs/time/0.2.16/src/time/lib.rs.html#94-127
const TIME_FORMAT: &'static str = "%a, %d %b %Y %H:%M:%S GMT";

/// A file with an associated name; responds with the Content-Type based on the
/// file extension.
#[derive(Debug)]
pub struct NamedFile(PathBuf, File);

impl NamedFile {
    /// Attempts to open a file in read-only mode.
    ///
    /// # Errors
    ///
    /// This function will return an error if path does not already exist. Other
    /// errors may also be returned according to
    /// [`OpenOptions::open()`](std::fs::OpenOptions::open()).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rocket::response::NamedFile;
    ///
    /// #[allow(unused_variables)]
    /// # rocket::async_test(async {
    /// let file = NamedFile::open("foo.txt").await;
    /// });
    /// ```
    pub async fn open<P: AsRef<Path>>(path: P) -> io::Result<NamedFile> {
        // FIXME: Grab the file size here and prohibit `seek`ing later (or else
        // the file's effective size may change), to save on the cost of doing
        // all of those `seek`s to determine the file size. But, what happens if
        // the file gets changed between now and then?
        let file = File::open(path.as_ref()).await?;
        Ok(NamedFile(path.as_ref().to_path_buf(), file))
    }

    /// Retrieve the underlying `File`.
    #[inline(always)]
    pub fn file(&self) -> &File {
        &self.1
    }

    /// Retrieve a mutable borrow to the underlying `File`.
    #[inline(always)]
    pub fn file_mut(&mut self) -> &mut File {
        &mut self.1
    }

    /// Take the underlying `File`.
    #[inline(always)]
    pub fn take_file(self) -> File {
        self.1
    }

    /// Retrieve the path of this file.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use std::io;
    /// use rocket::response::NamedFile;
    ///
    /// # #[allow(dead_code)]
    /// # async fn demo_path() -> io::Result<()> {
    /// let file = NamedFile::open("foo.txt").await?;
    /// assert_eq!(file.path().as_os_str(), "foo.txt");
    /// # Ok(())
    /// # }
    /// ```
    #[inline(always)]
    pub fn path(&self) -> &Path {
        self.0.as_path()
    }
}

/// Streams the named file to the client. Sets or overrides the Content-Type in
/// the response according to the file's extension if the extension is
/// recognized. See [`ContentType::from_extension()`] for more information. If
/// you would like to stream a file with a different Content-Type than that
/// implied by its extension, use a [`File`] directly. On supported platforms, this
/// will set or override the Last-Modified and header in the response according to the
/// file's modified date on disk and respond with 304 Not Modified to conditional
/// requests. See [`fs::Metadata::modified`] for more information on supported
/// platforms.
impl<'r> Responder<'r, 'static> for NamedFile {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        // using the file handle `self.1` is too much trouble here, tokio's `File::metadata` method
        // is async, so is into_std, and try_into_std gives you a `io::Result<std::fs::File>`
        let file_mod_time = fs::metadata(self.0.as_path()).ok()
            .and_then(|metadata| metadata.modified().ok())
            .and_then(|modified| modified.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|duration| OffsetDateTime::from_unix_timestamp(duration.as_secs() as i64));

        // if file_mod_time is < this date, we don't need to re-send the file.
        let cutoff = req.headers().get_one("If-Modified-Since")
            .and_then(|header| PrimitiveDateTime::parse(header, TIME_FORMAT).ok())
            .map(|dt| dt.assume_utc());

        if let Some((date, cutoff)) = file_mod_time.zip(cutoff) {
            if date <= cutoff {
                return response::status::NotModified.respond_to(req);
            }
        }

        let mut response = self.1.respond_to(req)?;
        if let Some(ext) = self.0.extension() {
            if let Some(ct) = ContentType::from_extension(&ext.to_string_lossy()) {
                response.set_header(ct);
            }
        }

        if let Some(time) = file_mod_time {
            response.set_raw_header("Last-Modified", time.format(TIME_FORMAT));
        }

        Ok(response)
    }
}

impl Deref for NamedFile {
    type Target = File;

    fn deref(&self) -> &File {
        &self.1
    }
}

impl DerefMut for NamedFile {
    fn deref_mut(&mut self) -> &mut File {
        &mut self.1
    }
}
