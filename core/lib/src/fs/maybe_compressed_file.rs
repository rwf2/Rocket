use std::ffi::OsString;
use std::path::Path;
use std::io;

use rocket_http::{ContentCoding, ContentEncoding, ContentType};

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
///     MaybeCompressedFile::open(path).await.ok()
/// }
/// ```
///
/// Always prefer to use [`FileServer`] which has more functionality and a
/// pithier API.
///
/// [`FileServer`]: crate::fs::FileServer
#[derive(Debug)]
pub(crate) struct MaybeCompressedFile {
    encoding: ContentCoding,
    ct_ext: Option<OsString>,
    file: NamedFile, 
}

impl MaybeCompressedFile {
    /// Attempts to open files in read-only mode.
    /// 
    /// # Errors
    ///
    /// This function will return an error if the selected path does not already 
    /// exist. Other errors may also be returned according to
    /// [`OpenOptions::open()`](std::fs::OpenOptions::open()).
    pub async fn open<P: AsRef<Path>>(encoding: ContentCoding, path: P) -> io::Result<MaybeCompressedFile> {
        let o_path = path.as_ref().to_path_buf();

        let (ct_ext, encoding, file) = match o_path.extension() {
            // A compressed file is being requested, no need to compress again.
            Some(e) if e == "gz" => 
                (Some(e.to_owned()), ContentCoding::IDENTITY, NamedFile::open(path).await?),
            
            ct_ext if encoding.is_gzip() => {
                // construct path to the compressed file
                let ct_ext = ct_ext.map(|e| e.to_owned());
                let zip_ext = ct_ext.as_ref().map(|e| {
                    let mut z = e.to_owned();
                    z.push(".gz");
                    z
                }).unwrap_or(OsString::from("gz"));

                let zipped = o_path.with_extension(zip_ext);
                match zipped.exists() {
                    true  => (ct_ext, encoding, NamedFile::open(zipped).await?),
                    false => (ct_ext, ContentCoding::IDENTITY, NamedFile::open(path).await?),
                }
            }

            // gzip not supported, fall back to IDENTITY
            ct_ext => 
                (ct_ext.map(|e| e.to_owned()), ContentCoding::IDENTITY, NamedFile::open(o_path).await?),
            
        };

        Ok(MaybeCompressedFile { ct_ext, encoding, file })
    }

    pub fn file(&self) -> &NamedFile {
        &self.file
    }
}

/// Streams the *appropriate* named file to the client. Sets or overrides the 
/// Content-Type in the response according to the file's non-zipped extension 
/// if appropriate and if the extension is recognized. See 
/// [`ContentType::from_extension()`] for more information. If you would like to
/// stream a file with a different Content-Type than that implied by its 
/// extension, use a [`File`] directly.
impl<'r> Responder<'r, 'static> for MaybeCompressedFile {
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'static> {
        let mut response = self.file.respond_to(request)?;

        if !self.encoding.is_identity() && !self.encoding.is_any() {
            response.set_header(ContentEncoding::from(self.encoding));
        }
        
        if let Some(ext) = self.ct_ext {
            if let Some(ct) = ContentType::from_extension(&ext.to_string_lossy()) {
                response.set_header(ct);
            }
        }
        
        Ok(response)
    }
}
