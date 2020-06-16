use proc_macro::{TokenStream, Span};
use devise::{syn, Spanned, Result};

use crate::proc_macro2::TokenStream as TokenStream2;
use crate::syn_ext::syn_to_diag;

trait EntryAttr {
    /// Whether the attribute requires the attributed function to be `async`.
    const REQUIRES_ASYNC: bool;

    /// Return a new or rewritten function, using block as the main execution.
    fn function(f: &syn::ItemFn, body: &syn::Block) -> Result<TokenStream2>;
}

struct Main;

impl EntryAttr for Main {
    const REQUIRES_ASYNC: bool = true;

    fn function(f: &syn::ItemFn, block: &syn::Block) -> Result<TokenStream2> {
        let (vis, mut sig) = (&f.vis, f.sig.clone());
        if sig.ident != "main" {
            return Err(Span::call_site()
                .error("attribute can only be applied to `main` function")
                .span_note(sig.span(), "this function must be `main`"));
        }

        sig.asyncness = None;
        Ok(quote_spanned!(block.span().into() => #vis #sig {
            ::rocket::async_main(async move #block)
        }))
    }
}

struct Test;

impl EntryAttr for Test {
    const REQUIRES_ASYNC: bool = true;

    fn function(f: &syn::ItemFn, block: &syn::Block) -> Result<TokenStream2> {
        let (vis, mut sig) = (&f.vis, f.sig.clone());
        sig.asyncness = None;
        Ok(quote_spanned!(block.span().into() => #[test] #vis #sig {
            ::rocket::async_test(async move #block)
        }))
    }
}

struct Launch;

impl EntryAttr for Launch {
    const REQUIRES_ASYNC: bool = false;

    fn function(f: &syn::ItemFn, block: &syn::Block) -> Result<TokenStream2> {
        if f.sig.ident == "main" {
            return Err(Span::call_site()
                .error("attribute cannot be applied to `main` function")
                .note("this attribute generates a `main` function")
                .span_note(f.sig.span(), "this function cannot be `main`"));
        }

        let ty = match &f.sig.output {
            syn::ReturnType::Type(_, ty) => ty,
            _ => return Err(Span::call_site()
                .error("attribute can only be applied to functions that return a value")
                .span_note(f.sig.span(), "this function must return a value"))
        };

        let (vis, mut sig) = (&f.vis, f.sig.clone());
        sig.ident = syn::Ident::new("main", sig.ident.span());
        sig.output = syn::ReturnType::Default;
        sig.asyncness = None;

        let rocket = quote_spanned!(ty.span().into() => {
            let ___rocket: #ty = #block;
            let ___rocket: ::rocket::Rocket = ___rocket;
            ___rocket
        });

        if f.sig.asyncness.is_some() {
            Ok(quote_spanned!(block.span().into() => #vis #sig {
                ::rocket::async_main(async move {
                    let _ = #rocket.launch().await;
                })
            } #[allow(dead_code)] #f))
        } else {
            Ok(quote_spanned!(block.span().into() => #vis #sig {
                ::rocket::async_main({
                    let ___rocket = #rocket;
                    async move { let _ = ___rocket.launch().await; }
                })
            } #[allow(dead_code)] #f))
        }
    }
}

fn parse_input<A: EntryAttr>(input: TokenStream) -> Result<syn::ItemFn> {
    let function: syn::ItemFn = syn::parse(input)
        .map_err(syn_to_diag)
        .map_err(|d| d.help("attribute can only be applied to functions"))?;

    if A::REQUIRES_ASYNC && function.sig.asyncness.is_none() {
        return Err(Span::call_site()
            .error("attribute can only be applied to `async` functions")
            .span_note(function.sig.span(), "this function must be `async`"));
    }

    if !function.sig.inputs.is_empty() {
        return Err(Span::call_site()
            .error("attribute can only be applied to functions without arguments")
            .span_note(function.sig.span(), "this function must take no arguments"));
    }

    Ok(function)
}

fn _async_entry<A: EntryAttr>(_args: TokenStream, input: TokenStream) -> Result<TokenStream> {
    let function = parse_input::<A>(input)?;
    let (original_attrs, block) = (&function.attrs, &function.block);
    let new_fn = A::function(&function, block)?;
    Ok(quote!(#(#original_attrs)* #new_fn).into())
}

pub fn async_test_attribute(args: TokenStream, input: TokenStream) -> TokenStream {
    _async_entry::<Test>(args, input).unwrap_or_else(|d| { d.emit(); TokenStream::new() })
}

pub fn main_attribute(args: TokenStream, input: TokenStream) -> TokenStream {
    _async_entry::<Main>(args, input).unwrap_or_else(|d| { d.emit(); quote!(fn main() {}).into() })
}

pub fn launch_attribute(args: TokenStream, input: TokenStream) -> TokenStream {
    _async_entry::<Launch>(args, input).unwrap_or_else(|d| { d.emit(); quote!(fn main() {}).into() })
}
