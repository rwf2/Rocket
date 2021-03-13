use std::fmt::Display;

use devise::{syn, Result};
use devise::ext::{SpanDiagnosticExt, quote_respanned};

use crate::http::uri;
use crate::syn::{Expr, Ident, Type, spanned::Spanned};
use crate::http_codegen::Optional;
use crate::syn_ext::IdentExt;
use crate::bang::uri_parsing::*;
use crate::proc_macro2::TokenStream;
use crate::attribute::param::Parameter;
use crate::exports::_uri;

use crate::URI_MACRO_PREFIX;

macro_rules! p {
    (@go $num:expr, $singular:expr, $plural:expr) => (
        if $num == 1 { $singular.into() } else { $plural }
    );

    ("parameter", $n:expr) => (p!(@go $n, "parameter", "parameters"));
    ($n:expr, "was") => (p!(@go $n, "1 was", format!("{} were", $n)));
    ($n:expr, "parameter") => (p!(@go $n, "1 parameter", format!("{} parameters", $n)));
}

pub fn prefix_last_segment(path: &mut syn::Path, prefix: &str) {
    let mut last_seg = path.segments.last_mut().expect("syn::Path has segments");
    last_seg.ident = last_seg.ident.prepend(prefix);
}

pub fn _uri_macro(input: TokenStream) -> Result<TokenStream> {
    let input2: TokenStream = input.clone();
    let mut params = syn::parse2::<UriParams>(input)?;
    prefix_last_segment(&mut params.route_path, URI_MACRO_PREFIX);

    let path = &params.route_path;
    Ok(quote!(#path!(#input2)))
}

fn extract_exprs(internal: &InternalUriParams) -> Result<(
        impl Iterator<Item = &Expr>,                // path exprs
        impl Iterator<Item = &ArgExpr>,             // query exprs
        impl Iterator<Item = (&Ident, &Type)>,      // types for both path || query
    )>
{
    let route_name = &internal.uri_params.route_path;
    match internal.validate() {
        Validation::Ok(exprs) => {
            let path_params = internal.dynamic_path_params();
            let path_param_count = path_params.clone().count();
            for expr in exprs.iter().take(path_param_count) {
                if expr.as_expr().is_none() {
                    return Err(expr.span().error("path parameters cannot be ignored"));
                }
            }

            let query_exprs = exprs.clone().into_iter().skip(path_param_count);
            let path_exprs = exprs.into_iter().map(|e| e.unwrap_expr()).take(path_param_count);
            let types = internal.fn_args.iter().map(|a| (&a.ident, &a.ty));
            Ok((path_exprs, query_exprs, types))
        }
        Validation::NamedIgnored(_) => {
            let mut route_name = quote!(#route_name).to_string();
            route_name.retain(|c| !c.is_whitespace());

            let diag = internal.uri_params.args_span()
                .error("expected unnamed arguments due to ignored parameters")
                .note(format!("uri for route `{}` ignores path parameters: \"{}\"",
                        route_name, internal.route_uri));

            Err(diag)
        }
        Validation::Unnamed(expected, actual) => {
            let mut route_name = quote!(#route_name).to_string();
            route_name.retain(|c| !c.is_whitespace());

            let diag = internal.uri_params.args_span()
                .error(format!("expected {} but {} supplied",
                         p!(expected, "parameter"), p!(actual, "was")))
                .note(format!("route `{}` has uri \"{}\"",
                        route_name, internal.route_uri));

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
                let spans: Vec<_> = extra.iter().map(|ident| ident.span()).collect();
                diag = diag.span_help(spans, format!("unknown {}: {}", ps, msg));
            }

            if !dup.is_empty() {
                let (ps, msg) = join(dup.iter());
                let spans: Vec<_> = dup.iter().map(|ident| ident.span()).collect();
                diag = diag.span_help(spans, format!("duplicate {}: {}", ps, msg));
            }

            Err(diag)
        }
    }
}

fn add_binding<P: uri::UriPart>(to: &mut Vec<TokenStream>, ident: &Ident, ty: &Type, expr: &Expr) {
    let span = expr.span();
    let part = match P::KIND {
        uri::Kind::Path => quote_spanned!(span => #_uri::Path),
        uri::Kind::Query  => quote_spanned!(span => #_uri::Query),
    };

    let tmp_ident = ident.clone().with_span(expr.span());
    let let_stmt = quote_spanned!(span => let #tmp_ident = #expr);

    to.push(quote_spanned!(span =>
        #[allow(non_snake_case)] #let_stmt;
        let #ident = <#ty as #_uri::FromUriParam<#part, _>>::from_uri_param(#tmp_ident);
    ));
}

fn explode_path<'a>(
    internal: &InternalUriParams,
    bindings: &mut Vec<TokenStream>,
    mut exprs: impl Iterator<Item = &'a Expr>,
    mut args: impl Iterator<Item = (&'a Ident, &'a Type)>,
) -> TokenStream {
    if internal.dynamic_path_params().count() == 0 {
        let route_uri = &internal.route_uri;
        if let Some(ref mount) = internal.uri_params.mount_point {
            let full_uri = route_uri.map_path(|p| format!("{}/{}", mount, p))
                .expect("origin from path")
                .into_normalized();

            let path = full_uri.path().as_str();
            return quote!(#_uri::UriArgumentsKind::Static(#path));
        } else {
            let path = route_uri.path().as_str();
            return quote!(#_uri::UriArgumentsKind::Static(#path));
        }
    }

    let uri_display = quote!(#_uri::UriDisplay<#_uri::Path>);
    let all_path_params = internal.mount_params.iter().chain(internal.path_params.iter());
    let dyn_exprs = all_path_params.map(|param| {
        match param {
            Parameter::Static(name) => {
                quote!(&#name as &dyn #uri_display)
            },
            Parameter::Dynamic(_) | Parameter::Guard(_) => {
                let (ident, ty) = args.next().expect("ident/ty for non-ignored");
                let expr = exprs.next().expect("one expr per dynamic arg");
                add_binding::<uri::Path>(bindings, &ident, &ty, &expr);
                quote_spanned!(expr.span() => &#ident as &dyn #uri_display)
            }
            Parameter::Ignored(_) => {
                let expr = exprs.next().expect("one expr per dynamic arg");
                quote_spanned!(expr.span() => &#expr as &dyn #uri_display)
            }
        }
    });

    quote!(#_uri::UriArgumentsKind::Dynamic(&[#(#dyn_exprs),*]))
}

fn explode_query<'a>(
    internal: &InternalUriParams,
    bindings: &mut Vec<TokenStream>,
    mut arg_exprs: impl Iterator<Item = &'a ArgExpr>,
    mut args: impl Iterator<Item = (&'a Ident, &'a Type)>,
) -> Option<TokenStream> {
    let query = internal.route_uri.query()?.as_str();
    if internal.dynamic_query_params().count() == 0 {
        return Some(quote!(#_uri::UriArgumentsKind::Static(#query)));
    }

    let query_arg = quote!(#_uri::UriQueryArgument);
    let uri_display = quote!(#_uri::UriDisplay<#_uri::Query>);
    let dyn_exprs = internal.query_params.iter().filter_map(|param| {
        if let Parameter::Static(source) = param {
            return Some(quote!(#query_arg::Raw(#source)));
        }

        let dynamic = match param {
            Parameter::Static(source) =>  {
                return Some(quote!(#query_arg::Raw(#source)));
            },
            Parameter::Dynamic(ref seg) => seg,
            Parameter::Guard(ref seg) => seg,
            Parameter::Ignored(_) => unreachable!("invariant: unignorable q")
        };

        let (ident, ty) = args.next().expect("ident/ty for query");
        let arg_expr = arg_exprs.next().expect("one expr per query");
        let expr = match arg_expr.as_expr() {
            Some(expr) => expr,
            None => {
                // Force a typecheck for the `Ignoreable` trait. Note that write
                // out the path to `is_ignorable` to get the right span.
                bindings.push(quote_respanned! { arg_expr.span() =>
                    rocket::http::uri::assert_ignorable::<#_uri::Query, #ty>();
                });

                return None;
            }
        };

        let name = &dynamic.name;
        add_binding::<uri::Query>(bindings, &ident, &ty, &expr);
        Some(match dynamic.trailing {
            false => quote_spanned! { expr.span() =>
                #query_arg::NameValue(#name, &#ident as &dyn #uri_display)
            },
            true => quote_spanned! { expr.span() =>
                #query_arg::Value(&#ident as &dyn #uri_display)
            },
        })
    });

    Some(quote!(#_uri::UriArgumentsKind::Dynamic(&[#(#dyn_exprs),*])))
}

pub fn _uri_internal_macro(input: TokenStream) -> Result<TokenStream> {
    // Parse the internal invocation and the user's URI param expressions.
    let internal = syn::parse2::<InternalUriParams>(input)?;
    let (path_exprs, query_exprs, mut fn_args) = extract_exprs(&internal)?;

    let mut bindings = vec![];
    let uri_mod = quote!(rocket::http::uri);
    let path = explode_path(&internal, &mut bindings, path_exprs, &mut fn_args);
    let query = explode_query(&internal, &mut bindings, query_exprs, fn_args);
    let query = Optional(query);

     Ok(quote!({
         #(#bindings)*
         #uri_mod::UriArguments { path: #path, query: #query, }.into_origin()
     }))
}
