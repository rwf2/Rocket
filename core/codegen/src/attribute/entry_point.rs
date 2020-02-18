use proc_macro::{TokenStream, Span};
use devise::{syn, Result};

use crate::syn_ext::syn_to_diag;

fn parse_input(input: TokenStream, attr_name: &str) -> Result<syn::ItemFn> {
    let function: syn::ItemFn = syn::parse(input).map_err(syn_to_diag)
        .map_err(|diag| diag.help(format!("`#[{}]` can only be applied to async functions", attr_name)))?;

    if function.sig.asyncness.is_none() {
        return Err(Span::call_site().error(format!("`#[{}]` can only be applied to async functions", attr_name)))
    }

    if !function.sig.inputs.is_empty() {
        return Err(Span::call_site().error(format!("`#[{}]` can only be applied to functions with no parameters", attr_name)));
    }

    Ok(function)
}

pub fn _entry_point(_args: TokenStream, input: TokenStream, attr_name: &str, is_test: bool) -> Result<TokenStream> {
    let function = parse_input(input, attr_name)?;

    let attrs = &function.attrs;
    let vis = &function.vis;
    let name = &function.sig.ident;
    let output = &function.sig.output;
    let body = &function.block;

    let (test_attr, fn_name) = if is_test {
        (Some(quote! { #[test] } ), quote! { async_test })
    } else {
        (None, quote! { async_main })
    };

    Ok(quote! {
        #test_attr
        #(#attrs)*
        #vis fn #name() #output {
            rocket :: #fn_name (async move {
                #body
            })
        }
    }.into())
}

pub fn async_test_attribute(args: TokenStream, input: TokenStream) -> TokenStream {
    _entry_point(args, input, "async_test", true).unwrap_or_else(|d| { d.emit(); TokenStream::new() })
}

pub fn main_attribute(args: TokenStream, input: TokenStream) -> TokenStream {
    _entry_point(args, input, "main", false).unwrap_or_else(|d| { d.emit(); TokenStream::new() })
}
