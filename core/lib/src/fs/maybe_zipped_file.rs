use std::ffi::OsString;
use std::path::Path;
use std::io;

use rocket_http::ContentType;

use crate::fs::NamedFile;
use crate::response::{self, Responder};
use crate::Request;

/// A [`Responder`] that looks for pre-zipped files on the filesystem (by an 
/// extra `.gz' file extension) to serve in place of the given file.
///
/// # Example
///
/// A simple static file server mimicking [`FileServer`]:
///
/// ```rust
/// # use rocket::get;
/// use std::path::{PathBuf, Path};
///
/// use rocket::fs::{NamedFile, relative};
///
/// #[get("/file/<path..>")]
/// pub async fn second(path: PathBuf) -> Option<NamedFile> {
///     let mut path = Path::new(relative!("static")).join(path);
///     if path.is_dir() {
///         path.push("index.html");
///     }
///
///     MaybeZippedFile::open(path).await.ok()
/// }
/// ```
///
/// Always prefer to use [`FileServer`] which has more functionality and a
/// pithier API.
///
/// [`FileServer`]: crate::fs::FileServer
#[derive(Debug)]
pub(crate) struct MaybeZippedFile {
    ct_ext: Option<OsString>,
    encoded: bool,
    file: NamedFile, 
}

impl MaybeZippedFile {
    /// Attempts to open files in read-only mode.
    /// 
    /// # Errors
    ///
    /// This function will return an error if the selected path does not already 
    /// exist. Other errors may also be returned according to
    /// [`OpenOptions::open()`](std::fs::OpenOptions::open()).
    pub async fn open<P: AsRef<Path>>(path: P) -> io::Result<MaybeZippedFile> {
        let o_path = path.as_ref().to_path_buf();

        let (ct_ext, encoded, file) = match o_path.extension() {
            // A gz file is being requested, no need to compress again.
            Some(e) if e == "gz" => (Some(e.to_owned()), false, NamedFile::open(path).await?),
            // construct path to the .gz file
            Some(e) => {
                let ct_ext = Some(e.to_owned());
                let mut zip_ext = e.to_owned();
                zip_ext.push(".gz");

                let zipped = o_path.with_extension(&zip_ext);
                match zipped.exists() {
                    true  => (ct_ext, true, NamedFile::open(zipped).await?),
                    false => (ct_ext, false, NamedFile::open(path).await?),
                }
            }
            // construct path to the .gz file
            None => {
                let zipped = o_path.with_extension("gz");
                match zipped.exists() {
                    true  => (None, true, NamedFile::open(zipped).await?),
                    false => (None, false, NamedFile::open(path).await?),
                }
            }
        };

        Ok(MaybeZippedFile { ct_ext, encoded, file })
    }
}

/// Streams the *appropriate* named file to the client. Sets or overrides the 
/// Content-Type in the response according to the file's non-zipped extension 
/// if appropriate and if the extension is recognized. See 
/// [`ContentType::from_extension()`] for more information. If you would like to
/// stream a file with a different Content-Type than that implied by its 
/// extension, use a [`File`] directly.
impl<'r> Responder<'r, 'static> for MaybeZippedFile {
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'static> {
        let mut response = self.file.respond_to(request)?;

        if self.encoded {
            response.set_raw_header("Content-Encoding", "gzip");
        }
        
        if let Some(ext) = self.ct_ext {
            if let Some(ct) = ContentType::from_extension(&ext.to_string_lossy()) {
                response.set_header(ct);
            }
        }
        
        Ok(response)
    }
}
