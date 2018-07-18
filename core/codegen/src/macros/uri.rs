use std::fmt::Display;

use syntax::codemap::Span;
use syntax::ext::base::{DummyResult, ExtCtxt, MacEager, MacResult};
use syntax::tokenstream::{TokenStream, TokenTree};
use syntax::ast::{self, GenericArg, MacDelimiter, Ident};
use syntax::symbol::Symbol;
use syntax::parse::PResult;
use syntax::ext::build::AstBuilder;
use syntax::ptr::P;

use URI_INFO_MACRO_PREFIX;
use super::prefix_path;
use utils::{IdentExt, split_idents, ExprExt};
use parser::{UriParams, InternalUriParams, Validation};

pub fn uri(
    ecx: &mut ExtCtxt,
    sp: Span,
    args: &[TokenTree],
) -> Box<MacResult + 'static> {
    // Generate the path to the internal macro.
    let mut parser = ecx.new_parser_from_tts(args);
    let (_, mut path) = try_parse!(sp, UriParams::parse_prelude(ecx, &mut parser));
    prefix_path(URI_INFO_MACRO_PREFIX, &mut path);

    // It's incredibly important we use `sp` as the Span for the generated code
    // so that errors from the `internal` call show up on the user's code.
    let expr = parser.mk_mac_expr(sp,
        ast::Mac_ {
            path,
            delim: MacDelimiter::Parenthesis,
            tts: args.to_vec().into_iter().collect::<TokenStream>().into(),
        },
        ::syntax::util::ThinVec::new(),
    );

    MacEager::expr(expr)
}

fn extract_exprs<'a>(
    ecx: &ExtCtxt<'a>,
    internal: &InternalUriParams,
) -> PResult<'a, Vec<P<ast::Expr>>> {
    let route_name = &internal.uri_params.route_path;
    match internal.validate() {
        Validation::Ok(exprs) => Ok(exprs),
        Validation::Unnamed(expected, actual) => {
            let mut diag = ecx.struct_span_err(internal.uri_params.args_span(),
                &format!("`{}` route uri expects {} but {} supplied",
                         route_name, p!(expected, "parameter"), p!(actual, "was")));

            if expected > 0 {
                let ps = p!("parameter", expected);
                diag.note(&format!("expected {}: {}", ps, internal.fn_args_str()));
            }

            Err(diag)
        }
        Validation::Named(missing, extra, dup) => {
            let e = &format!("invalid parameters for `{}` route uri", route_name);
            let mut diag = ecx.struct_span_err(internal.uri_params.args_span(), e);
            diag.note(&format!("uri parameters are: {}", internal.fn_args_str()));

            fn join<S: Display, T: Iterator<Item = S>>(iter: T) -> (&'static str, String) {
                let items: Vec<_> = iter.map(|i| format!("`{}`", i)).collect();
                (p!("parameter", items.len()), items.join(", "))
            }

            if !extra.is_empty() {
                let (ps, msg) = join(extra.iter().map(|id| id.node));
                let spans: Vec<_> = extra.iter().map(|ident| ident.span).collect();
                diag.span_help(spans, &format!("unknown {}: {}", ps, msg));
            }

            if !dup.is_empty() {
                let (ps, msg) = join(dup.iter().map(|id| id.node));
                let spans: Vec<_> = dup.iter().map(|ident| ident.span).collect();
                diag.span_help(spans, &format!("duplicate {}: {}", ps, msg));
            }

            if !missing.is_empty() {
                let (ps, msg) = join(missing.iter());
                diag.help(&format!("missing {}: {}", ps, msg));
            }

            Err(diag)
        }
    }
}

#[allow(unused_imports)]
pub fn uri_internal(
    ecx: &mut ExtCtxt,
    sp: Span,
    tt: &[TokenTree],
) -> Box<MacResult + 'static> {
    // Parse the internal invocation and the user's URI param expressions.
    let mut parser = ecx.new_parser_from_tts(tt);
    let internal = try_parse!(sp, InternalUriParams::parse(ecx, &mut parser));
    let exprs = try_parse!(sp, extract_exprs(ecx, &internal));

    // Generate the statements to typecheck each parameter. First, the mount.
    let mut argument_stmts = vec![];
    let mut format_assign_tokens = vec![];
    let mut fmt_string = internal.uri_fmt_string();
    if let Some(mount_point) = internal.uri_params.mount_point {
        // generating: let mount: &str = $mount_string;
        let mount_string = mount_point.node;
        argument_stmts.push(ecx.stmt_let_typed(
            mount_point.span,
            false,
            Ident::from_str("mount"),
            quote_ty!(ecx, &str),
            quote_expr!(ecx, $mount_string),
        ));

        // generating: format string arg for `mount`
        let mut tokens = quote_tokens!(ecx, mount = mount,);
        tokens.iter_mut().for_each(|tree| tree.set_span(mount_point.span));
        format_assign_tokens.push(tokens);

        // Ensure the `format!` string contains the `{mount}` parameter.
        fmt_string = "{mount}".to_string() + &fmt_string;
    }

    // Now the user's parameters.
    // Building <$T as ::rocket::http::uri::FromUriParam<_>>::from_uri_param($e).
    for (i, &(mut ident, ref ty)) in internal.fn_args.iter().enumerate() {
        let (span, mut expr) = (exprs[i].span, exprs[i].clone());

        // Format argument names cannot begin with `_`, but a function parameter
        // might, so we prefix each parameter with the letters `fmt`.
        ident.name = Symbol::intern(&format!("fmt{}", ident.name));
        ident.span = span;

        // path for call: <T as FromUriParam<_>>::from_uri_param
        let idents = split_idents("rocket::http::uri::FromUriParam");
        let generics = vec![GenericArg::Type(ecx.ty(span, ast::TyKind::Infer))];
        let trait_path = ecx.path_all(span, true, idents, generics, vec![]);
        let method = Ident::new(Symbol::intern("from_uri_param"), span);
        let (qself, path) = ecx.qpath(ty.clone(), trait_path, method);

        // replace &expr with [let tmp = expr; &tmp] so that borrows of
        // temporary expressions live at least as long as the call to
        // `from_uri_param`. Otherwise, exprs like &S { .. } won't compile.
        let cloned_expr = expr.clone().into_inner();
        if let ast::ExprKind::AddrOf(_, inner) = cloned_expr.node {
            // Only reassign temporary expressions, not locations.
            if !inner.is_location() {
                let tmp_ident = ident.append("_tmp");
                let tmp_stmt = ecx.stmt_let(span, false, tmp_ident, inner);
                argument_stmts.push(tmp_stmt);
                expr = ecx.expr_ident(span, tmp_ident);
            }
        }

        // generating: let $ident = path($expr);
        let path_expr = ecx.expr_qpath(span, qself, path);
        let call = ecx.expr_call(span, path_expr, vec![expr]);
        let stmt = ecx.stmt_let(span, false, ident, call);
        debug!("Emitting URI typecheck statement: {:?}", stmt);
        argument_stmts.push(stmt);

        // generating: arg assignment tokens for format string
        let uri_display = quote_path!(ecx, ::rocket::http::uri::UriDisplay);
        let mut tokens = quote_tokens!(ecx, $ident = &$ident as &$uri_display,);
        tokens.iter_mut().for_each(|tree| tree.set_span(span));
        format_assign_tokens.push(tokens);
    }

    let expr = quote_expr!(ecx, {
        $argument_stmts
        ::rocket::http::uri::Uri::from(format!($fmt_string, $format_assign_tokens))
    });

    debug!("Emitting URI expression: {:?}", expr);
    MacEager::expr(expr)
}
