mod uri;

use {ROUTE_STRUCT_PREFIX, CATCH_STRUCT_PREFIX};
use utils::{sep_by_tok, ParserExt, IdentExt};

use syntax::codemap::Span;
use syntax::tokenstream::TokenTree;
use syntax::ast::{Path, Expr};
use syntax::ext::base::{DummyResult, ExtCtxt, MacResult, MacEager};
use syntax::parse::token::Token;
use syntax::ptr::P;

pub use self::uri::{uri, uri_internal};

#[inline]
pub fn prefix_path(prefix: &str, path: &mut Path) {
    let last = path.segments.len() - 1;
    let last_seg = &mut path.segments[last];
    last_seg.identifier = last_seg.identifier.prepend(prefix);
}

#[inline]
pub fn prefix_paths(prefix: &str, paths: &mut Vec<Path>) {
    for p in paths {
        prefix_path(prefix, p);
    }
}

pub fn prefixing_vec_macro<F>(
    prefix: &str,
    mut to_expr: F,
    ecx: &mut ExtCtxt,
    sp: Span,
    args: &[TokenTree]) -> Box<MacResult + 'static>
where F: FnMut(&ExtCtxt, Path) -> P<Expr>
{
    let mut parser = ecx.new_parser_from_tts(args);
    match parser.parse_paths() {
        Ok(mut paths) => {
            // Prefix each path terminator and build up the P<Expr> for each path.
            prefix_paths(prefix, &mut paths);
            let path_exprs: Vec<P<Expr>> = paths.into_iter()
                .map(|path| to_expr(ecx, path))
                .collect();

            // Now put them all in one vector and return the thing.
            let path_list = sep_by_tok(ecx, &path_exprs, Token::Comma);
            let output = quote_expr!(ecx, vec![$path_list]).into_inner();
            MacEager::expr(P(output))
        }
        Err(mut e) => {
            e.emit();
            DummyResult::expr(sp)
        }
    }
}

#[rustfmt_skip]
pub fn routes(ecx: &mut ExtCtxt, sp: Span, args: &[TokenTree])
        -> Box<MacResult + 'static> {
    prefixing_vec_macro(ROUTE_STRUCT_PREFIX, |ecx, path| {
        quote_expr!(ecx, ::rocket::Route::from(&$path))
    }, ecx, sp, args)
}

#[rustfmt_skip]
pub fn catchers(ecx: &mut ExtCtxt, sp: Span, args: &[TokenTree])
        -> Box<MacResult + 'static> {
    prefixing_vec_macro(CATCH_STRUCT_PREFIX, |ecx, path| {
        quote_expr!(ecx, ::rocket::Catcher::from(&$path))
    }, ecx, sp, args)
}
