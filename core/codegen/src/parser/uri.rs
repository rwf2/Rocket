use syntax::ast::*;
use syntax::codemap::{Span, Spanned, dummy_spanned};
use syntax::ext::base::ExtCtxt;

use rocket_http::uri::Uri;
use super::route::param_to_ident;
use utils::{span, SpanExt, is_valid_ident};

// We somewhat arbitrarily enforce absolute paths. This is mostly because we
// want the initial "/" to represent the mount point. Empty segments are
// stripped out at runtime. So, to avoid any confusion, we issue an error at
// compile-time for empty segments. At the moment, this disallows trailing
// slashes as well, since then the last segment is empty.
fn valid_path(ecx: &ExtCtxt, uri: &Uri, sp: Span) -> bool {
    let cleaned = uri.to_string();
    if !uri.as_str().starts_with('/') {
        ecx.struct_span_err(sp, "route paths must be absolute")
            .note(&format!("expected {:?}, found {:?}", cleaned, uri.as_str()))
            .emit()
    } else if cleaned != uri.as_str() {
        ecx.struct_span_err(sp, "paths cannot contain empty segments")
            .note(&format!("expected {:?}, found {:?}", cleaned, uri.as_str()))
            .emit()
    } else {
        return true;
    }

    false
}

fn valid_segments(ecx: &ExtCtxt, uri: &Uri, sp: Span) -> bool {
    let mut validated = true;
    let mut segments_span = None;
    for segment in uri.segments() {
        // We add one to the index to account for the '/'.
        let index = segment.as_ptr() as usize - uri.path().as_ptr() as usize;
        let span = sp.trim_left(index + 1).shorten_to(segment.len());

        // If we're iterating after a '..' param, that's a hard error.
        if let Some(span) = segments_span {
            let rem_sp = sp.trim_left(index).trim_right(1);
            ecx.struct_span_err(rem_sp, "text after a trailing '..' param")
                .help("a segments param must be the final text in a path")
                .span_note(span, "trailing param is here")
                .emit();
            return false;
        }

        // Check if this is a dynamic param. If so, check it's well-formedness.
        if segment.starts_with('<') && segment.ends_with('>') {
            let mut param = &segment[1..(segment.len() - 1)];
            if segment.ends_with("..>") {
                segments_span = Some(span);
                param = &param[..(param.len() - 2)];
            }

            if param.is_empty() {
                ecx.span_err(span, "parameters cannot be empty");
            } else if !is_valid_ident(param) {
                ecx.struct_span_err(span, "parameter names must be valid identifiers")
                    .note(&format!("{:?} is not a valid identifier", param))
                    .emit();
            } else if param == "_" {
                ecx.struct_span_err(span, "parameters must be named")
                    .help("use a name such as `_guard` or `_param`")
                    .emit();
            } else {
                continue
            }

            validated = false;
        } else if segment.starts_with('<') {
            if segment[1..].contains('<') || segment.contains('>') {
                ecx.struct_span_err(span, "malformed parameter")
                    .help("parameters must be of the form '<param>'")
                    .emit();
            } else {
                ecx.struct_span_err(span, "parameter is missing a closing bracket")
                    .help(&format!("perhaps you meant '{}>'?", segment))
                    .emit();
            }

            validated = false;
        } else if Uri::percent_encode(segment) != segment {
            if segment.contains('<') || segment.contains('>') {
                ecx.struct_span_err(span, "malformed parameter")
                    .help("parameters must be of the form '<param>'")
                    .emit();
            } else {
                ecx.span_err(span, "segment contains invalid characters");
            }

            validated = false;
        }
    }

    validated
}

pub fn validate_uri(ecx: &ExtCtxt,
                    string: &str,
                    sp: Span)
                    -> (Spanned<Uri<'static>>, Option<Spanned<Ident>>) {
    let uri = Uri::from(string.to_string());
    let query_param = string.find('?')
        .map(|i| span(&string[(i + 1)..], sp.trim_left(i + 1)))
        .and_then(|spanned_q_param| param_to_ident(ecx, spanned_q_param));

    if valid_segments(ecx, &uri, sp) && valid_path(ecx, &uri, sp) {
        (span(uri, sp), query_param)
    } else {
        (dummy_spanned(Uri::new("")), query_param)
    }
}
