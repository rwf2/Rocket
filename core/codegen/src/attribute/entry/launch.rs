use super::EntryAttr;

use devise::{Spanned, Result};
use devise::ext::SpanDiagnosticExt;
use proc_macro2::{TokenStream, Span};

/// `#[rocket::launch]`: generates a `main` function that calls the attributed
/// function to generate a `Rocket` instance. Then calls `.launch()` on the
/// returned instance inside of an `rocket::async_main`.
pub struct Launch;

impl EntryAttr for Launch {
    const REQUIRES_ASYNC: bool = false;

    fn function(f: &mut syn::ItemFn) -> Result<TokenStream> {
        if f.sig.ident == "main" {
            return Err(Span::call_site()
                .error("attribute cannot be applied to `main` function")
                .note("this attribute generates a `main` function")
                .span_note(f.sig.ident.span(), "this function cannot be `main`"));
        }

        // Always infer the type as `Rocket<Build>`.
        if let syn::ReturnType::Type(_, ref mut ty) = &mut f.sig.output {
            if let syn::Type::Infer(_) = &mut **ty {
                let new = quote_spanned!(ty.span() => ::rocket::Rocket<::rocket::Build>);
                *ty = syn::parse2(new).expect("path is type");
            }
        }

        let ty = match &f.sig.output {
            syn::ReturnType::Type(_, ty) => ty,
            _ => return Err(Span::call_site()
                .error("attribute can only be applied to functions that return a value")
                .span_note(f.sig.span(), "this function must return a value"))
        };

        let block = &f.block;
        let rocket = quote_spanned!(ty.span() => {
            let ___rocket: #ty = #block;
            let ___rocket: ::rocket::Rocket<::rocket::Build> = ___rocket;
            ___rocket
        });

        let (vis, mut sig) = (&f.vis, f.sig.clone());
        sig.ident = syn::Ident::new("main", sig.ident.span());
        sig.output = syn::ReturnType::Default;
        sig.asyncness = None;

        Ok(quote_spanned!(block.span() =>
            #[allow(dead_code)] #f

            #vis #sig {
                ::rocket::async_main(async move { let _res = #rocket.launch().await; })
            }
        ))
    }
}
