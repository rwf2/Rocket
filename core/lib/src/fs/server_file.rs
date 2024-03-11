use std::io;

use time::OffsetDateTime;

use crate::fs::MaybeCompressedFile;
use crate::response::{self, Responder};
use crate::Request;

/// A [`Responder`] that wraps the [`MaybeCompressedFile`] and sets the 
/// `Last-Modified` header.
///
/// [`FileServer`]: crate::fs::FileServer
#[derive(Debug)]
pub(crate) struct ServerFile {
    last_modified: Option<String>,
    file: MaybeCompressedFile, 
}

impl ServerFile {
    /// Attempts to read file metadata.
    /// 
    /// # Errors
    ///
    /// This function will return an error if the file's metadata cannot be read.
    /// [`OpenOptions::open()`](std::fs::OpenOptions::open()).
    pub async fn new(file: io::Result<MaybeCompressedFile>) -> io::Result<Self> {
        let file = file?;
        let metadata = file.file().metadata().await?;
        let last_modified = metadata.modified()?.duration_since(std::time::UNIX_EPOCH).ok()
            .and_then(|d| i64::try_from(d.as_secs()).ok())
            .and_then(|sec| OffsetDateTime::from_unix_timestamp(sec).ok())
            .and_then(|odt| odt.format(&time::format_description::well_known::Rfc2822).ok());
        
        Ok(Self { last_modified, file })
    }
}

/// Sets the last-modified data for the file response
impl<'r> Responder<'r, 'static> for ServerFile {
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'static> {
        let mut response = self.file.respond_to(request)?;

        if let Some(last_modified) = self.last_modified {
            response.set_raw_header("last-modified", last_modified);
        }

        Ok(response)
    }
}
