use proc_macro::{TokenStream, Span};
use devise::{syn, Result};

use crate::proc_macro2::TokenStream as TokenStream2;
use crate::syn_ext::syn_to_diag;

#[derive(Copy, Clone)]
enum Kind {
    Main,
    Test,
}

impl Kind {
    // The name of the attribute, used for error messages
    fn attr_name(&self) -> &'static str {
        match self {
            Kind::Main => "main",
            Kind::Test => "async_test",
        }
    }

    // Attributes to decorate the generated function with
    fn attrs(&self) -> Option<TokenStream2> {
        match self {
            Kind::Main => None,
            Kind::Test => Some(quote!{ #[test] }),
        }
    }

    // The path to the function to call
    fn fn_path(&self) -> TokenStream2 {
        match self {
            Kind::Main => quote! { rocket :: async_main },
            Kind::Test => quote! { rocket :: async_test },
        }
    }
}

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

fn _async_entry(_args: TokenStream, input: TokenStream, kind: Kind) -> Result<TokenStream> {
    let function = parse_input(input, kind.attr_name())?;

    let attrs = &function.attrs;
    let vis = &function.vis;
    let name = &function.sig.ident;
    let output = &function.sig.output;
    let body = &function.block;

    let test_attr = kind.attrs();
    let fn_path = kind.fn_path();

    Ok(quote! {
        #test_attr
        #(#attrs)*
        #vis fn #name() #output {
            #fn_path (async move {
                #body
            })
        }
    }.into())
}

pub fn async_test_attribute(args: TokenStream, input: TokenStream) -> TokenStream {
    _async_entry(args, input, Kind::Test).unwrap_or_else(|d| { d.emit(); TokenStream::new() })
}

pub fn main_attribute(args: TokenStream, input: TokenStream) -> TokenStream {
    _async_entry(args, input, Kind::Main).unwrap_or_else(|d| { d.emit(); TokenStream::new() })
}
