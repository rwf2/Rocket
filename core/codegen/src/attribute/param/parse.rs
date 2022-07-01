use unicode_xid::UnicodeXID;
use devise::{Diagnostic, ext::SpanDiagnosticExt};
use proc_macro2::Span;

use crate::name::Name;
use crate::proc_macro_ext::StringLit;
use crate::attribute::param::{Parameter, Dynamic};
use crate::http::uri::fmt::{Part, Kind, Path};

#[derive(Debug)]
pub struct Error<'a> {
    segment: &'a str,
    span: Span,
    source: &'a str,
    source_span: Span,
    kind: ErrorKind,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum ErrorKind {
    Empty,
    BadIdent,
    Ignored,
    EarlyTrailing,
    NoTrailing,
    Static,
}

impl Dynamic {
    pub fn parse<P: Part>(
        segment: &str,
        span: Span,
    ) -> Result<Self, Error<'_>>  {
        match Parameter::parse::<P>(&segment, span)? {
            Parameter::Dynamic(d) | Parameter::Ignored(d) => Ok(d),
            Parameter::Guard(g) => Ok(g.source),
            Parameter::Static(_) => Err(Error::new(segment, span, ErrorKind::Static)),
        }
    }
}

impl Parameter {
    pub fn parse<P: Part>(
        segment: &str,
        source_span: Span,
    ) -> Result<Self, Error<'_>>  {
        let mut trailing = false;

        // Check if this is a dynamic param. If so, check its well-formedness.
        if segment.starts_with('<') && segment.ends_with('>') {
            let mut name = &segment[1..(segment.len() - 1)];
            if name.ends_with("..") {
                trailing = true;
                name = &name[..(name.len() - 2)];
            }

            let span = subspan(name, segment, source_span);
            if name.is_empty() {
                return Err(Error::new(name, source_span, ErrorKind::Empty));
            } else if !is_valid_ident(name) {
                return Err(Error::new(name, span, ErrorKind::BadIdent));
            }

            let dynamic = Dynamic { name: Name::new(name, span), trailing, index: 0 };
            if dynamic.is_wild() && P::KIND != Kind::Path {
                return Err(Error::new(name, span, ErrorKind::Ignored));
            } else if dynamic.is_wild() {
                return Ok(Parameter::Ignored(dynamic));
            } else {
                return Ok(Parameter::Dynamic(dynamic));
            }
        } else if segment.is_empty() {
            return Err(Error::new(segment, source_span, ErrorKind::Empty));
        } else if segment.starts_with('<') {
            let candidate = candidate_from_malformed(segment);
            source_span.warning("`segment` starts with `<` but does not end with `>`")
                .help(format!("perhaps you meant the dynamic parameter `<{}>`?", candidate))
                .emit_as_item_tokens();
        } else if segment.contains('>') || segment.contains('<') {
            source_span.warning("`segment` contains `<` or `>` but is not a dynamic parameter")
                .emit_as_item_tokens();
        }

        Ok(Parameter::Static(Name::new(segment, source_span)))
    }

    pub fn parse_many<P: Part>(
        source: &str,
        source_span: Span,
    ) -> impl Iterator<Item = Result<Self, Error<'_>>> {
        let mut trailing: Option<(&str, Span)> = None;

        // We check for empty segments when we parse an `Origin` in `FromMeta`.
        source.split(P::DELIMITER)
            .filter(|s| !s.is_empty())
            .enumerate()
            .map(move |(i, segment)| {
                if let Some((trail, span)) = trailing {
                    let error = Error::new(trail, span, ErrorKind::EarlyTrailing)
                        .source(source, source_span);

                    return Err(error);
                }

                let segment_span = subspan(segment, source, source_span);
                let mut parsed = Self::parse::<P>(segment, segment_span)
                    .map_err(|e| e.source(source, source_span))?;

                if let Some(ref mut d) = parsed.dynamic_mut() {
                    if d.trailing {
                        trailing = Some((segment, segment_span));
                    }

                    d.index = i;
                }

                Ok(parsed)
            })
    }
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::Empty => "parameters cannot be empty".fmt(f),
            ErrorKind::BadIdent => "invalid identifier".fmt(f),
            ErrorKind::Ignored => "parameter must be named".fmt(f),
            ErrorKind::NoTrailing => "parameter cannot be trailing".fmt(f),
            ErrorKind::EarlyTrailing => "unexpected text after trailing parameter".fmt(f),
            ErrorKind::Static => "unexpected static parameter".fmt(f),
        }
    }
}

impl<'a> Error<'a> {
    pub fn new(segment: &str, span: Span, kind: ErrorKind) -> Error<'_> {
        Error { segment, source: segment, span, source_span: span, kind }
    }

    pub fn source(mut self, source: &'a str, span: Span) -> Self {
        self.source = source;
        self.source_span = span;
        self
    }
}

impl From<Error<'_>> for Diagnostic {
    fn from(error: Error<'_>) -> Self {
        match error.kind {
            ErrorKind::Empty => error.span.error(error.kind.to_string()),
            ErrorKind::BadIdent => {
                let candidate = candidate_from_malformed(error.segment);
                error.span.error(format!("{}: `{}`", error.kind, error.segment))
                    .help("dynamic parameters must be valid identifiers")
                    .help(format!("did you mean `<{}>`?", candidate))
            }
            ErrorKind::Ignored => {
                error.span.error(error.kind.to_string())
                    .help("use a name such as `_guard` or `_param`")
            }
            ErrorKind::EarlyTrailing => {
                trailspan(error.segment, error.source, error.source_span)
                    .error(error.kind.to_string())
                    .help("a trailing parameter must be the final component")
                    .span_note(error.span, "trailing param is here")
            }
            ErrorKind::NoTrailing => {
                let candidate = candidate_from_malformed(error.segment);
                error.span.error(error.kind.to_string())
                    .help(format!("did you mean `<{}>`?", candidate))
            }
            ErrorKind::Static => {
                let candidate = candidate_from_malformed(error.segment);
                error.span.error(error.kind.to_string())
                    .help(format!("parameter must be dynamic: `<{}>`", candidate))
            }
        }
    }
}

impl devise::FromMeta for Dynamic {
    fn from_meta(meta: &devise::MetaItem) -> devise::Result<Self> {
        let string = StringLit::from_meta(meta)?;
        let span = string.subspan(1..string.len() + 1);
        let param = Dynamic::parse::<Path>(&string, span)?;

        if param.is_wild() {
            return Err(Error::new(&string, span, ErrorKind::Ignored).into());
        } else if param.trailing {
            return Err(Error::new(&string, span, ErrorKind::NoTrailing).into());
        } else {
            Ok(param)
        }
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

fn candidate_from_malformed(segment: &str) -> String {
    let candidate = segment.chars().enumerate()
        .filter(|(i, c)| *i == 0 && is_ident_start(*c) || *i != 0 && is_ident_continue(*c))
        .map(|(_, c)| c)
        .collect::<String>();

    if candidate.is_empty() {
        "param".into()
    } else {
        candidate
    }
}

#[inline]
fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic()
        || c == '_'
        || (c > '\x7f' && UnicodeXID::is_xid_start(c))
}

#[inline]
fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || c == '_'
        || (c > '\x7f' && UnicodeXID::is_xid_continue(c))
}

fn is_valid_ident(string: &str) -> bool {
    let mut chars = string.chars();
    match chars.next() {
        Some(c) => is_ident_start(c) && chars.all(is_ident_continue),
        None => false
    }
}
