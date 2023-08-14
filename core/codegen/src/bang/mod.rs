mod uri;
mod uri_parsing;
mod test_guide;
mod export;

pub mod typed_stream;

use devise::Result;
use syn::{Path, punctuated::Punctuated, parse::Parser, Token};
use syn::spanned::Spanned;
use proc_macro2::TokenStream;

fn struct_maker_vec(
    input: proc_macro::TokenStream,
    ty: TokenStream,
    map: impl Fn(TokenStream) -> TokenStream,
) -> Result<TokenStream> {
    use crate::exports::_Vec;

    // Parse a comma-separated list of paths.
    let paths = <Punctuated<Path, Token![,]>>::parse_terminated.parse(input)?;
    let exprs = paths.iter().map(|path| {
        let expr = map(quote_spanned!(path.span() => ___struct));
        quote_spanned!(path.span() => {
            let ___struct = #path {};
            let ___item: #ty = #expr;
            ___item
        })
    });

    Ok(quote!({
        let ___vec: #_Vec<#ty> = vec![#(#exprs),*];
        ___vec
    }))
}

pub fn routes_macro(input: proc_macro::TokenStream) -> TokenStream {
    struct_maker_vec(input, quote!(::rocket::Route), |e| quote!(#e.into_route()))
        .unwrap_or_else(|diag| diag.emit_as_expr_tokens())
}

pub fn catchers_macro(input: proc_macro::TokenStream) -> TokenStream {
    struct_maker_vec(input, quote!(::rocket::Catcher), |e| quote!(#e.into_catcher()))
        .unwrap_or_else(|diag| diag.emit_as_expr_tokens())
}

pub fn uri_macro(input: proc_macro::TokenStream) -> TokenStream {
    uri::_uri_macro(input.into())
        .unwrap_or_else(|diag| diag.emit_as_expr_tokens_or(quote! {
            rocket::http::uri::Origin::ROOT
        }))
}

pub fn uri_internal_macro(input: proc_macro::TokenStream) -> TokenStream {
    // TODO: Ideally we would generate a perfect `Origin::ROOT` so that we don't
    // assist in propagating further errors. Alas, we can't set the span to the
    // invocation of `uri!` without access to `span.parent()`, and
    // `Span::call_site()` here points to the `#[route]`, immediate caller,
    // generating a rather confusing error message when there's a type-mismatch.
    uri::_uri_internal_macro(input.into())
        .unwrap_or_else(|diag| diag.emit_as_expr_tokens_or(quote! {
            rocket::http::uri::Origin::ROOT
        }))
}

pub fn guide_tests_internal(input: proc_macro::TokenStream) -> TokenStream {
    test_guide::_macro(input)
        .unwrap_or_else(|diag| diag.emit_as_item_tokens())
}

pub fn export_internal(input: proc_macro::TokenStream) -> TokenStream {
    export::_macro(input)
        .unwrap_or_else(|diag| diag.emit_as_item_tokens())
}

pub fn typed_stream(input: proc_macro::TokenStream) -> TokenStream {
    typed_stream::_macro(input)
        .unwrap_or_else(|diag| diag.emit_as_item_tokens())
}
