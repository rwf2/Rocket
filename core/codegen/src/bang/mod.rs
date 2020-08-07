use devise::Result;

use crate::syn_ext::IdentExt;
use crate::syn::{Path, punctuated::Punctuated, parse::Parser, Token};
use crate::syn::spanned::Spanned;
use crate::proc_macro2::TokenStream;
use crate::{ROUTE_STRUCT_PREFIX, CATCH_STRUCT_PREFIX};

mod uri;
mod uri_parsing;
mod test_guide;

pub fn prefix_last_segment(path: &mut Path, prefix: &str) {
    let mut last_seg = path.segments.last_mut().expect("syn::Path has segments");
    last_seg.ident = last_seg.ident.prepend(prefix);
}

fn _prefixed_vec(
    prefix: &str,
    input: proc_macro::TokenStream,
    ty: &TokenStream
) -> Result<TokenStream> {
    // Parse a comma-separated list of paths.
    let mut paths = <Punctuated<Path, Token![,]>>::parse_terminated.parse(input)?;

    // Prefix the last segment in each path with `prefix`.
    paths.iter_mut().for_each(|p| prefix_last_segment(p, prefix));

    // Return a `vec!` of the prefixed, mapped paths.
    let prefixed_mapped_paths = paths.iter()
        .map(|path| quote_spanned!(path.span().into() => #ty::from(&#path)));

    Ok(quote!(vec![#(#prefixed_mapped_paths),*]))
}

fn prefixed_vec(
    prefix: &str,
    input: proc_macro::TokenStream,
    ty: TokenStream
) -> TokenStream {
    define_vars_and_mods!(_Vec);
    _prefixed_vec(prefix, input, &ty)
        .map(|vec| quote!({
            let __vector: #_Vec<#ty> = #vec;
            __vector
        }))
        .unwrap_or_else(|diag| {
            let diag_tokens = diag.emit_as_expr_tokens();
            quote!({
                #diag_tokens
                let __vec: #_Vec<#ty> = vec![];
                __vec
            })
        })
}

pub fn routes_macro(input: proc_macro::TokenStream) -> TokenStream {
    prefixed_vec(ROUTE_STRUCT_PREFIX, input, quote!(::rocket::Route))
}

pub fn catchers_macro(input: proc_macro::TokenStream) -> TokenStream {
    prefixed_vec(CATCH_STRUCT_PREFIX, input, quote!(::rocket::Catcher))
}

pub fn uri_macro(input: proc_macro::TokenStream) -> TokenStream {
    uri::_uri_macro(input.into())
        .unwrap_or_else(|diag| diag.emit_as_expr_tokens())
}

pub fn uri_internal_macro(input: proc_macro::TokenStream) -> TokenStream {
    let tokens = uri::_uri_internal_macro(input.into())
        .unwrap_or_else(|diag| diag.emit_as_expr_tokens());
    tokens
}

pub fn guide_tests_internal(input: proc_macro::TokenStream) -> TokenStream {
    test_guide::_macro(input)
        .unwrap_or_else(|diag| diag.emit_as_item_tokens())
}
