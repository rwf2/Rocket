use std::fmt::Display;
use proc_macro::TokenStream;
use crate::proc_macro2::TokenStream as TokenStream2;

use devise::{syn, Result};
use devise::syn::{Expr, Ident, Type, spanned::Spanned};
use crate::http::{uri::{Origin, Path, Query}, ext::IntoOwned};
use crate::http::route::{RouteSegment, Kind, Source};

use crate::http_codegen::Optional;
use crate::syn_ext::{IdentExt, syn_to_diag};
use crate::bang::{prefix_last_segment, uri_parsing::*};

use crate::URI_MACRO_PREFIX;

macro_rules! p {
    (@go $num:expr, $singular:expr, $plural:expr) => (
        if $num == 1 { $singular.into() } else { $plural }
    );

    ("parameter", $n:expr) => (p!(@go $n, "parameter", "parameters"));
    ($n:expr, "was") => (p!(@go $n, "1 was", format!("{} were", $n)));
    ($n:expr, "parameter") => (p!(@go $n, "1 parameter", format!("{} parameters", $n)));
}

crate fn _uri_macro(input: TokenStream) -> Result<TokenStream> {
    let input2: TokenStream2 = input.clone().into();
    let mut params = syn::parse::<UriParams>(input).map_err(syn_to_diag)?;
    prefix_last_segment(&mut params.route_path, URI_MACRO_PREFIX);

    let path = &params.route_path;
    Ok(quote!(#path!(#input2)).into())
}

fn extract_exprs<'a>(internal: &'a InternalUriParams) -> Result<(
        impl Iterator<Item = (&'a Ident, &'a Type, &'a Expr)>,
        impl Iterator<Item = (&'a Ident, &'a Type, &'a ArgExpr)>,
    )>
{
    let route_name = &internal.uri_params.route_path;
    match internal.validate() {
        Validation::Ok(exprs) => {
            let path_param_count = internal.route_uri.path().matches('<').count();
            for expr in exprs.iter().take(path_param_count) {
                if !expr.as_expr().is_some() {
                    return Err(expr.span().unstable()
                               .error("path parameters cannot be ignored"));
                }
            }

            // Create an iterator over all `ident`, `ty`, and `expr` triples.
            let arguments = internal.fn_args.iter()
                .zip(exprs.into_iter())
                .map(|(FnArg { ident, ty }, expr)| (ident, ty, expr));

            // Create iterators for just the path and query parts.
            let path_params = arguments.clone()
                .take(path_param_count)
                .map(|(i, t, e)| (i, t, e.unwrap_expr()));

            let query_params = arguments.skip(path_param_count);
            Ok((path_params, query_params))
        }
        Validation::Unnamed(expected, actual) => {
            let mut diag = internal.uri_params.args_span().error(
                format!("`{}` route uri expects {} but {} supplied", quote!(#route_name),
                         p!(expected, "parameter"), p!(actual, "was")));

            if expected > 0 {
                let ps = p!("parameter", expected);
                diag = diag.note(format!("expected {}: {}", ps, internal.fn_args_str()));
            }

            Err(diag)
        }
        Validation::Named(missing, extra, dup) => {
            let e = format!("invalid parameters for `{}` route uri", quote!(#route_name));
            let mut diag = internal.uri_params.args_span().error(e)
                .note(format!("uri parameters are: {}", internal.fn_args_str()));

            fn join<S: Display, T: Iterator<Item = S>>(iter: T) -> (&'static str, String) {
                let mut items: Vec<_> = iter.map(|i| format!("`{}`", i)).collect();
                items.dedup();
                (p!("parameter", items.len()), items.join(", "))
            }

            if !missing.is_empty() {
                let (ps, msg) = join(missing.iter());
                diag = diag.help(format!("missing {}: {}", ps, msg));
            }

            if !extra.is_empty() {
                let (ps, msg) = join(extra.iter());
                let spans: Vec<_> = extra.iter().map(|ident| ident.span().unstable()).collect();
                diag = diag.span_help(spans, format!("unknown {}: {}", ps, msg));
            }

            if !dup.is_empty() {
                let (ps, msg) = join(dup.iter());
                let spans: Vec<_> = dup.iter().map(|ident| ident.span().unstable()).collect();
                diag = diag.span_help(spans, format!("duplicate {}: {}", ps, msg));
            }

            Err(diag)
        }
    }
}

fn add_binding(to: &mut Vec<TokenStream2>, ident: &Ident, ty: &Type, expr: &Expr, source: Source) {
    let uri_mod = quote!(rocket::http::uri);
    let (span, ident_tmp) = (expr.span(), ident.prepend("tmp_"));
    let from_uri_param = if source == Source::Query {
        quote_spanned!(span => #uri_mod::FromUriParam<#uri_mod::Query, _>)
    } else {
        quote_spanned!(span => #uri_mod::FromUriParam<#uri_mod::Path, _>)
    };

    to.push(quote_spanned!(span =>
        let #ident_tmp = #expr;
        let #ident = <#ty as #from_uri_param>::from_uri_param(#ident_tmp);
    ));
}

fn explode_path<'a, I: Iterator<Item = (&'a Ident, &'a Type, &'a Expr)>>(
    uri: &Origin<'_>,
    bindings: &mut Vec<TokenStream2>,
    mut items: I
) -> TokenStream2 {
    let (uri_mod, path) = (quote!(rocket::http::uri), uri.path());
    if !path.contains('<') {
        return quote!(#uri_mod::UriArgumentsKind::Static(#path));
    }

    let uri_display = quote!(#uri_mod::UriDisplay<#uri_mod::Path>);
    let dyn_exprs = <RouteSegment<'_, Path>>::parse(uri).map(|segment| {
        let segment = segment.expect("segment okay; prechecked on parse");
        match segment.kind {
            Kind::Static => {
                let string = &segment.string;
                quote!(&#string as &dyn #uri_display)
            }
            Kind::Single | Kind::Multi => {
                let (ident, ty, expr) = items.next().expect("one item for each dyn");
                add_binding(bindings, &ident, &ty, &expr, Source::Path);
                quote_spanned!(expr.span() => &#ident as &dyn #uri_display)
            }
        }
    });

    quote!(#uri_mod::UriArgumentsKind::Dynamic(&[#(#dyn_exprs),*]))
}

fn explode_query<'a, I: Iterator<Item = (&'a Ident, &'a Type, &'a ArgExpr)>>(
    uri: &Origin<'_>,
    bindings: &mut Vec<TokenStream2>,
    mut items: I
) -> Option<TokenStream2> {
    let (uri_mod, query) = (quote!(rocket::http::uri), uri.query()?);
    if !query.contains('<') {
        return Some(quote!(#uri_mod::UriArgumentsKind::Static(#query)));
    }

    let query_arg = quote!(#uri_mod::UriQueryArgument);
    let uri_display = quote!(#uri_mod::UriDisplay<#uri_mod::Query>);
    let dyn_exprs = <RouteSegment<'_, Query>>::parse(uri)?.filter_map(|segment| {
        let segment = segment.expect("segment okay; prechecked on parse");
        if segment.kind == Kind::Static {
            let string = &segment.string;
            return Some(quote!(#query_arg::Raw(#string)));
        }

        let (ident, ty, arg_expr) = items.next().expect("one item for each dyn");
        let expr = match arg_expr.as_expr() {
            Some(expr) => expr,
            None => {
                // Force a typecheck for the `Ignoreable` trait. Note that write
                // out the path to `is_ignorable` to get the right span.
                bindings.push(quote_spanned! { arg_expr.span() =>
                    rocket::http::uri::assert_ignorable::<#uri_mod::Query, #ty>();
                });

                return None;
            }
        };

        let name = &segment.name;
        add_binding(bindings, &ident, &ty, &expr, Source::Query);
        Some(match segment.kind {
            Kind::Single => quote_spanned! { expr.span() =>
                #query_arg::NameValue(#name, &#ident as &dyn #uri_display)
            },
            Kind::Multi => quote_spanned! { expr.span() =>
                #query_arg::Value(&#ident as &dyn #uri_display)
            },
            Kind::Static => unreachable!("Kind::Static returns early")
        })
    });

    Some(quote!(#uri_mod::UriArgumentsKind::Dynamic(&[#(#dyn_exprs),*])))
}

// Returns an Origin URI with the mount point and route path concatinated. The
// query string is mangled by replacing single dynamic parameters in query parts
// (`<param>`) with `param=<param>`.
fn build_origin(internal: &InternalUriParams) -> Origin<'static> {
    let mount_point = internal.uri_params.mount_point.as_ref()
        .map(|origin| origin.path())
        .unwrap_or("");

    let path = format!("{}/{}", mount_point, internal.route_uri.path());
    let query = internal.route_uri.query();
    Origin::new(path, query).to_normalized().into_owned()
}

crate fn _uri_internal_macro(input: TokenStream) -> Result<TokenStream> {
    // Parse the internal invocation and the user's URI param expressions.
    let internal = syn::parse::<InternalUriParams>(input).map_err(syn_to_diag)?;
    let (path_params, query_params) = extract_exprs(&internal)?;

    let mut bindings = vec![];
    let uri = build_origin(&internal);
    let uri_mod = quote!(rocket::http::uri);
    let path = explode_path(&uri, &mut bindings, path_params);
    let query = Optional(explode_query(&uri, &mut bindings, query_params));

     Ok(quote!({
         #(#bindings)*
         #uri_mod::UriArguments { path: #path, query: #query, }.into_origin()
     }).into())
}
