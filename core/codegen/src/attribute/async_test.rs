use proc_macro::{TokenStream, Span};
use devise::{syn, Result};

use crate::syn_ext::syn_to_diag;

fn parse_input(input: TokenStream) -> Result<syn::ItemFn> {
    let function: syn::ItemFn = syn::parse(input).map_err(syn_to_diag)
        .map_err(|diag| diag.help("`#[async_test]` can only be applied to async functions"))?;

    if function.sig.asyncness.is_none() {
        return Err(Span::call_site().error("`#[async_test]` can only be applied to async functions"))
    }

    // TODO.async: verify of the form `async fn name(/* no args */) -> R`

    Ok(function)
}

pub fn _async_test(_args: TokenStream, input: TokenStream) -> Result<TokenStream> {
    let function = parse_input(input)?;

    let attrs = &function.attrs;
    let vis = &function.vis;
    let name = &function.sig.ident;
    let output = &function.sig.output;
    let body = &function.block;

    Ok(quote! {
        #[test]
        #(#attrs)*
        #vis fn #name() #output {
            rocket::async_test(async move {
                #body
            })
        }
    }.into())
}

pub fn async_test_attribute(args: TokenStream, input: TokenStream) -> TokenStream {
    _async_test(args, input).unwrap_or_else(|d| { d.emit(); TokenStream::new() })
}
