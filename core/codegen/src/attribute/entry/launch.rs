use super::EntryAttr;

use devise::{Spanned, Result};
use devise::ext::SpanDiagnosticExt;
use proc_macro2::{TokenStream, Span};

/// `#[rocket::launch]`: generates a `main` function that calls the attributed
/// function to generate a `Rocket` instance. Then calls `.launch()` on the
/// returned instance inside of an `rocket::async_main`.
pub struct Launch;

/// Determines if `f` likely spawns an async task, returning the spawn call.
fn likely_spawns(f: &syn::ItemFn) -> Option<&syn::ExprCall> {
    use syn::visit::{self, Visit};

    struct SpawnFinder<'a>(Option<&'a syn::ExprCall>);

    impl<'ast> Visit<'ast> for SpawnFinder<'ast> {
        fn visit_expr_call(&mut self, i: &'ast syn::ExprCall) {
            if self.0.is_some() {
                return;
            }

            if let syn::Expr::Path(ref e) = *i.func {
                let mut segments = e.path.segments.clone();
                if let Some(last) = segments.pop() {
                    if last.value().ident != "spawn" {
                        return visit::visit_expr_call(self, i);
                    }

                    if let Some(prefix) = segments.pop() {
                        if prefix.value().ident == "tokio" {
                            self.0 = Some(i);
                            return;
                        }
                    }

                    if let Some(syn::Expr::Async(_)) = i.args.first() {
                        self.0 = Some(i);
                        return;
                    }
                }
            };

            visit::visit_expr_call(self, i);
        }
    }

    let mut v = SpawnFinder(None);
    v.visit_item_fn(f);
    v.0
}

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

        let launch = match f.sig.asyncness {
            Some(_) => quote_spanned!(ty.span() => async move { #rocket.launch().await }),
            None => quote_spanned!(ty.span() => #rocket.launch()),
        };

        if f.sig.asyncness.is_none() {
            if let Some(call) = likely_spawns(f) {
                call.span()
                    .warning("task is being spawned outside an async context")
                    .span_help(f.sig.span(), "declare this function as `async fn` \
                                              to require async execution")
                    .span_note(Span::call_site(), "`#[launch]` call is here")
                    .emit_as_expr_tokens();
            }
        }

        let (vis, mut sig) = (&f.vis, f.sig.clone());
        sig.ident = syn::Ident::new("main", sig.ident.span());
        sig.output = syn::ReturnType::Default;
        sig.asyncness = None;

        Ok(quote_spanned!(block.span() =>
            #[allow(dead_code)] #f

            #vis #sig {
                let _ = ::rocket::async_main(#launch);
            }
        ))
    }
}
