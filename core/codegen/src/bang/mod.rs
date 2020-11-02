use devise::Result;

use crate::syn::{Path, punctuated::Punctuated, parse::Parser, Token};
use crate::proc_macro2::TokenStream;
use crate::syn::spanned::Spanned;

mod uri;
mod uri_parsing;
mod test_guide;

fn struct_maker_vec(
    input: proc_macro::TokenStream,
    ty: TokenStream,
) -> Result<TokenStream> {
    define_vars_and_mods!(_Vec);

    // Parse a comma-separated list of paths.
    let paths = <Punctuated<Path, Token![,]>>::parse_terminated.parse(input)?;
    let exprs = paths.iter()
        .map(|path| quote_spanned!(path.span() => {
            let ___struct = #path {};
            let ___item: #ty = ___struct.into();
            ___item
        }));

    Ok(quote!({
        let ___vec: #_Vec<#ty> = vec![#(#exprs),*];
        ___vec
    }))
}

pub fn routes_macro(input: proc_macro::TokenStream) -> TokenStream {
    struct_maker_vec(input, quote!(::rocket::Route))
        .unwrap_or_else(|diag| diag.emit_as_expr_tokens())
}

pub fn catchers_macro(input: proc_macro::TokenStream) -> TokenStream {
    struct_maker_vec(input, quote!(::rocket::Catcher))
        .unwrap_or_else(|diag| diag.emit_as_expr_tokens())
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
