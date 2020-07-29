use std::hash::{Hash, Hasher};

use devise::{syn, Diagnostic, ext::SpanDiagnosticExt};
use crate::proc_macro2::Span;

use crate::http::uri::{self, UriPart};
use crate::http::route::RouteSegment;
use crate::proc_macro_ext::{Diagnostics, StringLit, PResult, DResult};

pub use crate::http::route::{Error, Kind};

#[derive(Debug, Clone)]
pub struct Segment {
    pub span: Span,
    pub kind: Kind,
    pub source: Source,
    pub name: String,
    pub index: Option<usize>,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Source {
    Path,
    Query,
    Data,
    Unknown,
}

impl Segment {
    fn from<P: UriPart>(segment: RouteSegment<'_, P>, span: Span) -> Segment {
        let source = match P::DELIMITER {
            '/' => Source::Path,
            '&' => Source::Query,
            _ => unreachable!("only paths and queries")
        };

        let (kind, index) = (segment.kind, segment.index);
        Segment { span, kind, source, index, name: segment.name.into_owned() }
    }

    pub fn is_wild(&self) -> bool {
        self.name == "_"
    }

    pub fn is_dynamic(&self) -> bool {
        match self.kind {
            Kind::Static => false,
            Kind::Single | Kind::Multi => true,
        }
    }
}

impl From<&syn::Ident> for Segment {
    fn from(ident: &syn::Ident) -> Segment {
        Segment {
            kind: Kind::Static,
            source: Source::Unknown,
            span: ident.span(),
            name: ident.to_string(),
            index: None,
        }
    }
}

impl PartialEq for Segment {
    fn eq(&self, other: &Segment) -> bool {
        self.name == other.name
    }
}

impl Eq for Segment {  }

impl Hash for Segment {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

fn subspan(needle: &str, haystack: &str, span: Span) -> Span {
    let index = needle.as_ptr() as usize - haystack.as_ptr() as usize;
    StringLit::new(haystack, span).subspan(index..index + needle.len())
}

fn trailspan(needle: &str, haystack: &str, span: Span) -> Span {
    let index = needle.as_ptr() as usize - haystack.as_ptr() as usize;
    let lit = StringLit::new(haystack, span);
    if needle.as_ptr() as usize > haystack.as_ptr() as usize {
        lit.subspan((index - 1)..)
    } else {
        lit.subspan(index..)
    }
}

fn into_diagnostic(
    segment: &str, // The segment that failed.
    source: &str,  // The haystack where `segment` can be found.
    span: Span,    // The `Span` of `Source`.
    error: &Error<'_>,  // The error.
) -> Diagnostic {
    let seg_span = subspan(segment, source, span);
    match error {
        Error::Empty => seg_span.error(error.to_string()),
        Error::Ident(_) => {
            seg_span.error(error.to_string())
                .help("parameter names must be valid identifiers")
        }
        Error::Ignored => {
            seg_span.error(error.to_string())
                .help("use a name such as `_guard` or `_param`")
        }
        Error::MissingClose => {
            seg_span.error(error.to_string())
                .help(format!("did you mean '{}>'?", segment))
        }
        Error::Malformed => {
            seg_span.error(error.to_string())
                .help("parameters must be of the form '<param>'")
                .help("identifiers cannot contain '<' or '>'")
        }
        Error::Uri => {
            seg_span.error(error.to_string())
                .note("components cannot contain reserved characters")
                .help("reserved characters include: '%', '+', '&', etc.")
        }
        Error::Trailing(multi) => {
            let multi_span = subspan(multi, source, span);
            trailspan(segment, source, span)
                .error(error.to_string())
                .help("a multi-segment param must be the final component")
                .span_note(multi_span, "multi-segment param is here")
        }
    }
}

pub fn parse_data_segment(segment: &str, span: Span) -> PResult<Segment> {
    <RouteSegment<'_, uri::Query>>::parse_one(segment)
        .map(|segment| {
            let mut seg = Segment::from(segment, span);
            seg.source = Source::Data;
            seg.index = Some(0);
            seg
        })
        .map_err(|e| into_diagnostic(segment, segment, span, &e))
}

pub fn parse_segments<P: UriPart>(
    string: &str,
    span: Span
) -> DResult<Vec<Segment>> {
    let mut segments = vec![];
    let mut diags = Diagnostics::new();

    for result in <RouteSegment<'_, P>>::parse_many(string) {
        match result {
            Ok(segment) => {
                let seg_span = subspan(&segment.string, string, span);
                segments.push(Segment::from(segment, seg_span));
            },
            Err((segment_string, error)) => {
                diags.push(into_diagnostic(segment_string, string, span, &error));
                if let Error::Trailing(..) = error {
                    break;
                }
            }
        }
    }

    diags.err_or(segments)
}
