
use std::io;
use std::path::{Path, PathBuf};
use std::ops::{Deref, DerefMut};

use enum_flags::EnumFlags;
use crate::{
    http::{ContentType, Status, Method, TypedHeaders},
    response::{Responder, Builder},
    {Request, response, Response},
    tokio::fs::File,
    tokio::io::{ AsyncSeek}
};

use std::io::{SeekFrom};


use crate::http::{
    header_names,
    RangeItemHeaderValue,
    RangeHeaderValue,
    RangeConditionHeaderValue,
    DateTimeOffset,
    EntityTagHeaderValue,
    ContentRangeHeaderValue};


/// A [`Responder`] that sends a file with a Content-Type based on its name.
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
pub struct StaticFile {
    path: PathBuf,
    file: File,
    content_type: Option<ContentType>,
    len: u64,
    last_modified: DateTimeOffset,
    etag: EntityTagHeaderValue,
    if_match_state: PreconditionState,
    if_none_match_state: PreconditionState,
    if_modified_since_state: PreconditionState,
    if_unmodified_since_state: PreconditionState,
    range: Option<RangeItemHeaderValue>,
    request_type: RequestType
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
    /// use rocket::fs::NamedFile;
    ///
    /// #[get("/")]
    /// async fn index() -> Option<NamedFile> {
    ///     NamedFile::open("index.html").await.ok()
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
            if_match_state: PreconditionState::Unspecified,
            if_none_match_state: PreconditionState::Unspecified,
            if_modified_since_state: PreconditionState::Unspecified,
            if_unmodified_since_state: PreconditionState::Unspecified,
            range: None,
            request_type: RequestType::Unspecified
        })
    }

    /// Retrieve the underlying `File`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rocket::fs::NamedFile;
    ///
    /// # async fn f() -> std::io::Result<()> {
    /// let named_file = NamedFile::open("index.html").await?;
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
    /// use rocket::fs::NamedFile;
    ///
    /// # async fn f() -> std::io::Result<()> {
    /// let mut named_file = NamedFile::open("index.html").await?;
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
    /// use rocket::fs::NamedFile;
    ///
    /// # async fn f() -> std::io::Result<()> {
    /// let named_file = NamedFile::open("index.html").await?;
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
    /// use rocket::fs::NamedFile;
    ///
    /// # async fn demo_path() -> std::io::Result<()> {
    /// let file = NamedFile::open("foo.txt").await?;
    /// assert_eq!(file.path().as_os_str(), "foo.txt");
    /// # Ok(())
    /// # }
    /// ```
    #[inline(always)]
    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    fn is_range_request(&self) -> bool {
        self.request_type.contains(RequestType::IsRange)
    }

    fn comprehend_request_headers(&mut self, req: &Request<'_>) {
        // ComputeIfMatch
        self.compute_if_match(req);

        // compute_if_modified_since
        self.compute_if_modified_since(req);

        // ComputeRange

        self.compute_range(req);

        // ComputeIfRange
        self.compute_if_range(req);
    }

    fn get_precondition_state(&self) -> PreconditionState {
        let mut max = PreconditionState::Unspecified;
        for i in [
            self.if_match_state, self.if_none_match_state,
            self.if_modified_since_state, self.if_unmodified_since_state] {
            if i > max {
                max = i;
            }
        }
        max
    }

    fn compute_if_match(&mut self, req: &Request<'_>) {
        let request_headers = req.headers().get_typed_headers();

        // 14.24 If-Match
        let if_match = request_headers.if_match();
        if !if_match.is_empty() {
            self.if_match_state = PreconditionState::PreconditionFailed;
            for etag in if_match {
                if etag == EntityTagHeaderValue::any() && etag.compare(&self.etag, true) {
                    self.if_match_state = PreconditionState::ShouldProcess;
                    break;
                }
            }
        }

        // 14.26 If-None-Match
        let if_none_match = request_headers.if_none_match();
        if !if_none_match.is_empty() {
            self.if_none_match_state = PreconditionState::ShouldProcess;

            for etag in if_none_match {
                if etag == EntityTagHeaderValue::any() || etag.compare(&self.etag, true) {
                    self.if_none_match_state = PreconditionState::NotModified;
                    break;
                }
            }
        }
    }

    fn compute_if_modified_since(&mut self, req: &Request<'_>) {
        let now = DateTimeOffset::now();

        let request_headers = req.headers().get_typed_headers();

        // 14.25 If-Modified-Since
        if let Some(if_modified_since) = request_headers.if_modified_since() {
            if if_modified_since <= now {
                self.if_modified_since_state = if if_modified_since < self.last_modified {
                    PreconditionState::ShouldProcess
                } else {
                    PreconditionState::NotModified
                }
            }
        }

        // 14.28 If-Unmodified-Since
        if let Some(if_unmodified_since) = request_headers.if_unmodified_since() {
            if if_unmodified_since <= now {
                self.if_unmodified_since_state = if if_unmodified_since >= self.last_modified {
                    PreconditionState::ShouldProcess
                } else {
                    PreconditionState::PreconditionFailed
                }
            }
        }
    }

    fn compute_if_range(&mut self, req: &Request<'_>) {
        if let Some(if_range_header) = req.headers().get_typed_headers().if_range() {
            match if_range_header {
                RangeConditionHeaderValue::LastModified(last_modified) => {
                    if self.last_modified > last_modified {
                        self.request_type = self.request_type - RequestType::IsRange;
                    }
                }
                RangeConditionHeaderValue::EntityTag(etag) => {
                    if !etag.compare(&self.etag, true) {
                        self.request_type = self.request_type - RequestType::IsRange;
                    }
                }
            }
        }
    }

    fn compute_range(&mut self, req: &Request<'_>) {
        if req.method() != Method::Get {
            return;
        }

        let (is_range_request, range) = self.parse_range(req, self.len);

        self.range = range;
        if is_range_request {
            self.request_type |= RequestType::IsRange
        } else {
            self.request_type -= RequestType::IsRange
        }
    }

    fn parse_range(&mut self, req: &Request<'_>, len: u64) -> (bool, Option<RangeItemHeaderValue>) {
        let raw_range_header = req.headers().get(header_names::RANGE).collect::<Vec<&str>>();

        if raw_range_header.is_empty() || raw_range_header.join("") == "" {
            // Range header's value is empty.
            return (false, None);
        }

        if raw_range_header.len() > 1 || raw_range_header.get(0).unwrap().find(",").is_some() {
            // Multiple ranges are not supported.

            // The spec allows for multiple ranges but we choose not to support them because the client may request
            // very strange ranges (e.g. each byte separately, overlapping ranges, etc.) that could negatively
            // impact the server. Ignore the header and serve the response normally.
            return (false, None);
        }

        let range_header: Option<RangeHeaderValue> = req.headers().get_typed_headers().range();
        if range_header.is_none() {
            // Range header's value is invalid.
            // Invalid
            return (false, None);
        }
        let range_header = range_header.unwrap();

        // Already verified above
        assert_eq!(1, range_header.ranges.len());
        let ranges = &range_header.ranges;
        if ranges.is_empty() {
            return (true, None);
        }

        if len == 0 {
            return (true, None);
        }

        let range = ranges.first()
            .map(|r| r.normalize(len))
            .unwrap_or_default();

        (range.is_some(), range)
    }

    fn apply_response_headers(&mut self, builder: &mut Builder<'_>, status: Status) {
        builder.status(status);
        if status.code < 400 {

            // these headers are returned for 200, 206, and 304
            // they are not returned for 412 and 416

            if let Some(ct) = &self.content_type {
                builder.header(ct.clone());
            }
            builder.raw_header(header_names::LAST_MODIFIED, self.last_modified.to_string());
            builder.raw_header(header_names::ETAG, self.etag.to_string());
            builder.raw_header(header_names::ACCEPT_RANGES, "bytes");
            builder.raw_header(header_names::CONNECTION, "keep-alive");
        }

        if status == Status::Ok {
            // this header is only returned here for 200
            // it already set to the returned range for 206
            // it is not returned for 304, 412, and 416
            builder.raw_header(header_names::CONTENT_LENGTH, self.len.to_string());
        }
    }

    fn send<'r>(self, req: &'r Request<'_>) -> response::Result<'static> {
        let mut response = self.file.respond_to(req)?;
        if let Some(ct) = self.content_type {
            response.set_header(ct);
        }
        Ok(response)
    }

    fn send_range<'r>(mut self, _req: &'r Request<'_>) -> response::Result<'static> {
        // do range
        if let Some(ref range) = self.range {
            let mut builder = Response::build();
            let from = range.from.unwrap();
            let to = range.to.unwrap();
            let length = to - from + 1;
            let content_range_header: ContentRangeHeaderValue = (from, to, self.len).into();
            builder.header(content_range_header);
            builder.raw_header(header_names::CONTENT_LENGTH, length.to_string());

            self.apply_response_headers(&mut builder, Status::PartialContent);

            if from > 0 {
                std::pin::Pin::new(&mut self.file)
                    .start_seek(SeekFrom::Start(from))
                    .unwrap();
            }

            builder.sized_body(length as usize, self.file);

            Ok(builder.finalize())
        } else {
            // 14.16 Content-Range - A server sending a response with status code 416 (Requested range not satisfiable)
            // SHOULD include a Content-Range field with a byte-range-resp-spec of "*". The instance-length specifies
            // the current length of the selected resource.  e.g. */length
            let mut builder = Response::build();
            builder.header(ContentRangeHeaderValue::from(self.len));

            self.apply_response_headers(&mut builder,Status::RangeNotSatisfiable);

            Ok(builder.finalize())
        }
    }
}

/// Streams the named file to the client. Sets or overrides the Content-Type in
/// the response according to the file's extension if the extension is
/// recognized. See [`ContentType::from_extension()`] for more information. If
/// you would like to stream a file with a different Content-Type than that
/// implied by its extension, use a [`File`] directly.
impl<'r> Responder<'r, 'static> for StaticFile {
    fn respond_to(mut self, req: &'r Request<'_>) -> response::Result<'static> {
        use PreconditionState::*;
        self.comprehend_request_headers(req);
        match self.get_precondition_state() {
            Unspecified | ShouldProcess=> {
                if req.method() == Method::Head {
                    Ok(Response::build()
                        .status(Status::Ok)
                        .finalize())
                } else{
                    if self.is_range_request() {
                        self.send_range(req)
                    } else {
                        self.send(req)
                    }
                }
            }
            NotModified => {
                Ok(Response::build()
                        .status(Status::NotModified)
                        .finalize())
            }
            PreconditionFailed => {
                Ok(Response::build()
                    .status(Status::PreconditionFailed)
                    .finalize())
            }
        }
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


#[repr(u8)]
#[derive(Copy, Clone, Eq, PartialEq, Debug, Ord, PartialOrd)]
enum PreconditionState
{
    Unspecified,
    NotModified,
    ShouldProcess,
    PreconditionFailed,
}

#[repr(u8)]
#[derive(EnumFlags, Copy, Clone, Eq, PartialEq)]
enum RequestType {
    Unspecified = 0,
    IsHead = 1,
    IsGet = 2,
    IsRange = 4,
}
