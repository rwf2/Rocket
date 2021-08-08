
use enum_flags::EnumFlags;

use crate::{
    http::{
        ContentType,
        Status,
        Method,
        TypedHeaders,

        header_names,
        RangeItemHeaderValue,
        RangeHeaderValue,
        RangeConditionHeaderValue,
        DateTimeOffset,
        EntityTagHeaderValue,
        ContentRangeHeaderValue
    },
    response::{Builder, Response},
    {Request, response}
};


pub struct StaticFileContext {
    len: u64,
    content_type: Option<ContentType>,
    etag: EntityTagHeaderValue,
    last_modified: DateTimeOffset,

    if_match_state: PreconditionState,
    if_none_match_state: PreconditionState,
    if_modified_since_state: PreconditionState,
    if_unmodified_since_state: PreconditionState,
    range: Option<RangeItemHeaderValue>,
    request_type: RequestType
}

impl From<(u64, Option<ContentType>, EntityTagHeaderValue, DateTimeOffset)> for StaticFileContext {
    fn from((len, content_type, etag, last_modified): (u64, Option<ContentType>, EntityTagHeaderValue, DateTimeOffset)) -> Self {
        Self {
            len, content_type, etag, last_modified,
            if_match_state: PreconditionState::Unspecified,
            if_none_match_state: PreconditionState::Unspecified,
            if_modified_since_state: PreconditionState::Unspecified,
            if_unmodified_since_state: PreconditionState::Unspecified,
            range: None,
            request_type: RequestType::Unspecified
        }
    }
}

impl StaticFileContext {
    /// Comprehend the request headers.
    pub fn comprehend_request_headers(&mut self, req: &Request<'_>) {
        // ComputeIfMatch
        self.compute_if_match(req);

        // compute_if_modified_since
        self.compute_if_modified_since(req);

        // ComputeRange

        self.compute_range(req);

        // ComputeIfRange
        self.compute_if_range(req);
    }

    /// proceed the response
    pub fn proceed<'r, F: FnOnce((&mut Builder, u64, u64))>(self, req: &'r Request<'_>, send: F) -> response::Result<'static> {
        use PreconditionState::*;
        match self.get_precondition_state() {
            Unspecified | ShouldProcess=> {
                if req.method() == Method::Head {
                    Ok(Response::build()
                        .status(Status::Ok)
                        .finalize())
                } else if self.is_range_request() {
                    self.send_range(req, send)
                } else {
                    self.send(req, send)
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

    pub fn get_precondition_state(&self) -> PreconditionState {
        let mut max = PreconditionState::Unspecified;
        let precondition_states = [
            self.if_match_state, self.if_none_match_state,
            self.if_modified_since_state, self.if_unmodified_since_state];
        for i in  precondition_states {
            if i > max {
                max = i;
            }
        }
        max
    }

    fn is_range_request(&self) -> bool {
        self.request_type.contains(RequestType::IsRange)
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
                        self.request_type -= RequestType::IsRange;
                    }
                }
                RangeConditionHeaderValue::EntityTag(etag) => {
                    if !etag.compare(&self.etag, true) {
                        self.request_type -= RequestType::IsRange;
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

        if raw_range_header.len() > 1 || raw_range_header.get(0).unwrap().contains(',') {
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

    fn apply_response_headers(&self, builder: &mut Builder<'_>, status: Status) {
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

    fn send<'r,  F: FnOnce((&mut Builder, u64, u64))>(self, _req: &'r Request<'_>, send: F) -> response::Result<'static> {
        let mut builder = Response::build();

        self.apply_response_headers(&mut builder, Status::Ok);


        let len = self.len;
        send((&mut builder, 0, len));

        Ok(builder.finalize())
    }

    fn send_range<'r, F: FnOnce((&mut Builder, u64, u64))>(self, _req: &'r Request<'_>, send: F) -> response::Result<'static> {
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

            send((&mut builder, from, length));

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


#[repr(u8)]
#[derive(Copy, Clone, Eq, PartialEq, Debug, Ord, PartialOrd)]
pub enum PreconditionState
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