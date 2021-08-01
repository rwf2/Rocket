//!
//! Defines constants for well-known HTTP headers.
//!
#![allow(missing_docs)]

pub const RANGE: &'static str = "Range";
pub const IF_RANGE: &'static str = "If-Range";
pub const IF_MODIFIED_SINCE: &'static str = "If-Modified-Since";
pub const IF_UNMODIFIED_SINCE: &'static str = "If-Unmodified-Since";
pub const IF_MATCH: &'static str = "If-Match";
pub const IF_NONE_MATCH: &'static str = "If-None-Match";
pub const CONTENT_RANGE: &'static str = "Content-Range";
pub const LAST_MODIFIED: &'static str = "Last-Modified";
pub const ETAG: &'static str = "ETag";
pub const ACCEPT_RANGES: &'static str = "Accept-Ranges";
pub const CONTENT_LENGTH: &'static str = "Content-Length";
pub const CONNECTION: &'static str = "Connection";
