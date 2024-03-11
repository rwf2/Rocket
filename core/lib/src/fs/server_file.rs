use std::io;

use http::header::IF_MODIFIED_SINCE;
use rocket_http::Status;
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
    last_modified: Option<OffsetDateTime>,
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
            .and_then(|sec| OffsetDateTime::from_unix_timestamp(sec).ok());
            // .and_then(|odt| odt.format(&time::format_description::well_known::Rfc2822).ok());
        
        Ok(Self { last_modified, file })
    }
}

/// Sets the last-modified data for the file response
impl<'r> Responder<'r, 'static> for ServerFile {
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'static> {
        let if_modified_since = request.headers().get_one(IF_MODIFIED_SINCE.as_str())
            .and_then(|v| time::OffsetDateTime::parse(v, &time::format_description::well_known::Rfc2822).ok());

        match (self.last_modified, if_modified_since) {
            (Some(lm), Some(ims)) if lm <= ims => 
                return crate::Response::build().status(Status::NotModified).ok(),
            _ => {}
        }

        let mut response = self.file.respond_to(request)?;

        self.last_modified
            .and_then(|odt| odt.format(&time::format_description::well_known::Rfc2822).ok())
            .map(|lm| response.set_raw_header("last-modified", lm));

        Ok(response)
    }
}
