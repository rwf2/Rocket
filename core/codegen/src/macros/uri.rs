use std::fmt::Display;

use syntax::codemap::Span;
use syntax::ext::base::{DummyResult, ExtCtxt, MacEager, MacResult};
use syntax::tokenstream::{TokenStream, TokenTree};
use syntax::ast::{self, Expr, GenericArg, MacDelimiter, Ident};
use syntax::symbol::Symbol;
use syntax::parse::PResult;
use syntax::ext::build::AstBuilder;
use syntax::ptr::P;

use URI_INFO_MACRO_PREFIX;
use super::prefix_path;
use utils::{IdentExt, split_idents, ExprExt, option_as_expr};
use parser::{UriParams, InternalUriParams, Validation};

use rocket_http::uri::Origin;
use rocket_http::ext::IntoOwned;

// What gets called when `uri!` is invoked. This just invokes the internal URI
// macro which calls the `uri_internal` function below.
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
        ::syntax::ThinVec::new(),
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

// Validates the mount path and the URI and returns a single Origin URI with
// both paths concatinated. Validation should always succeed since this macro
// can only be called if the route attribute succeed, which implies that the
// route URI was valid.
fn extract_origin<'a>(
    ecx: &ExtCtxt<'a>,
    internal: &InternalUriParams,
) -> PResult<'a, Origin<'static>> {
    let base_uri = match internal.uri_params.mount_point {
        Some(base) => Origin::parse(&base.node)
            .map_err(|_| ecx.struct_span_err(base.span, "invalid path URI"))?
            .into_owned(),
        None => Origin::dummy()
    };

    Origin::parse_route(&format!("{}/{}", base_uri, internal.uri.node))
        .map(|o| o.to_normalized().into_owned())
        .map_err(|_| ecx.struct_span_err(internal.uri.span, "invalid route URI"))
}

fn explode<I>(ecx: &ExtCtxt, route_str: &str, items: I) -> P<Expr>
    where I: Iterator<Item = (ast::Ident, P<ast::Ty>, P<Expr>)>
{
    // Generate the statements to typecheck each parameter.
    // Building <$T as ::rocket::http::uri::FromUriParam<_>>::from_uri_param($e).
    let mut let_bindings = vec![];
    let mut fmt_exprs = vec![];
    for (mut ident, ty, expr) in items {
        let (span, mut expr) = (expr.span, expr.clone());
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
                let_bindings.push(tmp_stmt);
                expr = ecx.expr_ident(span, tmp_ident);
            }
        }

        // generating: let $ident = path($expr);
        let path_expr = ecx.expr_qpath(span, qself, path);
        let call = ecx.expr_call(span, path_expr, vec![expr]);
        let stmt = ecx.stmt_let(span, false, ident, call);
        debug!("Emitting URI typecheck statement: {:?}", stmt);
        let_bindings.push(stmt);

        // generating: arg tokens for format string
        let mut tokens = quote_tokens!(ecx, &$ident as &::rocket::http::uri::UriDisplay,);
        tokens.iter_mut().for_each(|tree| tree.set_span(span));
        fmt_exprs.push(tokens);
    }

    // Convert all of the '<...>' into '{}'.
    let mut inside = false;
    let fmt_string: String = route_str.chars().filter_map(|c| {
        Some(match c {
            '<' => { inside = true; '{' }
            '>' => { inside = false; '}' }
            _ if !inside => c,
            _ => return None
        })
    }).collect();

    // Don't allocate if there are no formatting expressions.
    if fmt_exprs.is_empty() {
        quote_expr!(ecx, $fmt_string.into())
    } else {
        quote_expr!(ecx, { $let_bindings format!($fmt_string, $fmt_exprs).into() })
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
    let origin = try_parse!(sp, extract_origin(ecx, &internal));

    // Determine how many parameters there are in the URI path.
    let path_param_count = origin.path().matches('<').count();

    // Create an iterator over the `ident`, `ty`, and `expr` triple.
    let mut arguments = internal.fn_args
        .into_iter()
        .zip(exprs.into_iter())
        .map(|((ident, ty), expr)| (ident, ty, expr));

    // Generate an expression for both the path and query.
    let path = explode(ecx, origin.path(), arguments.by_ref().take(path_param_count));
    let query = option_as_expr(ecx, &origin.query().map(|q| explode(ecx, q, arguments)));

    // Generate the final `Origin` expression.
    let expr = quote_expr!(ecx, ::rocket::http::uri::Origin::new::<
                                   ::std::borrow::Cow<'static, str>,
                                   ::std::borrow::Cow<'static, str>,
                                 >($path, $query));

    debug!("Emitting URI expression: {:?}", expr);
    MacEager::expr(expr)
}
