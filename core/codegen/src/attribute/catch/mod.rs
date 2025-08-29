mod parse;

use devise::ext::{SpanDiagnosticExt, TypeExt};
use devise::{Spanned, Result};
use proc_macro2::{TokenStream, Span};
use syn::{Lifetime, TypeReference};

use crate::http_codegen::Optional;
use crate::syn_ext::{FnArgExt, IdentExt, ReturnTypeExt};
use crate::exports::*;

pub fn _catch(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream
) -> Result<TokenStream> {
    // Parse and validate all of the user's input.
    let catch = parse::Attribute::parse(args.into(), input)?;

    // Gather everything we'll need to generate the catcher.
    let user_catcher_fn = &catch.function;
    let user_catcher_fn_name = &catch.function.sig.ident;
    let vis = &catch.function.vis;
    let status_code = Optional(catch.status.map(|s| s.code));
    let deprecated = catch.function.attrs.iter().find(|a| a.path().is_ident("deprecated"));

    // This ensures that "Responder not implemented" points to the return type.
    let return_type_span = catch.function.sig.output.ty()
        .map(|ty| ty.span())
        .unwrap_or_else(Span::call_site);

    let from_error = catch.guards.iter().map(|g| {
        let name = g.fn_ident.rocketized();
        let ty = g.ty.with_replaced_lifetimes(Lifetime::new("'__r", g.ty.span()));
        quote_spanned!(g.span() =>
            let #name: #ty = match <
                #ty as #FromError<'__r>
            >::from_error(#__status, #__req, #__error).await {
                #_Ok(v) => v,
                #_Err(__e) => {
                    ::rocket::trace::info!(
                        name: "error",
                        target: concat!("rocket::codegen::route::", module_path!()),
                        parameter = stringify!(#name),
                        type_name = stringify!(#ty),
                        status = __e.code,
                        "error guard error"
                    );
                    return #_Err(__e);
                },
            };
        )
    });

    let error = catch.error.iter().map(|g| {
        let name = g.fn_ident.rocketized();
        let ty = g.ty.with_replaced_lifetimes(Lifetime::new("'__r", g.ty.span()));
        quote!(
            let #name: #ty = match #_catcher::downcast(#__error) {
                Some(v) => v,
                None => {
                    ::rocket::trace::error!(
                        downcast_to = stringify!(#ty),
                        error_name = #__error.name(),
                        "Failed to downcast error. This should never happen, please \
                        open an issue with details."
                    );
                    return #_Err(#Status::InternalServerError);
                },
            };
        )
    });

    let error_type = Optional(catch.error.as_ref().map(|g| {
        let ty = match &g.ty {
            syn::Type::Reference(TypeReference { mutability: None, elem, .. }) => {
                elem.as_ref().with_stripped_lifetimes()
            },
            _ => return Err(g.ty.span().error("invalid type, must be a reference")),
        };
        Ok(quote_spanned!(g.span() =>
            #_catcher::TypeId::of::<#ty>()
        ))
    }).transpose()?);

    // We append `.await` to the function call if this is `async`.
    let dot_await = catch.function.sig.asyncness
        .map(|a| quote_spanned!(a.span() => .await));

    let args = catch.function.sig.inputs.iter().map(|a| {
        let name = a.typed().unwrap().0.rocketized();
        quote!(#name)
    });

    let catcher_response = quote_spanned!(return_type_span => {
        let ___responder = #user_catcher_fn_name(#(#args),*) #dot_await;
        #_response::Responder::respond_to(___responder, #__req).map_err(|e| e.status())?
    });

    // Generate the catcher, keeping the user's input around.
    Ok(quote! {
        #user_catcher_fn

        #[doc(hidden)]
        #[allow(nonstandard_style)]
        /// Rocket code generated proxy structure.
        #deprecated #vis struct #user_catcher_fn_name {  }

        /// Rocket code generated proxy static conversion implementations.
        #[allow(nonstandard_style, deprecated, clippy::style)]
        impl #user_catcher_fn_name {
            fn into_info(self) -> #_catcher::StaticInfo {
                fn monomorphized_function<'__r>(
                    #__status: #Status,
                    #__error: &'__r dyn #TypedError<'__r>,
                    #__req: &'__r #Request<'_>
                ) -> #_catcher::BoxFuture<'__r> {
                    #_Box::pin(async move {
                        #(#from_error)*
                        #(#error)*
                        let __response = #catcher_response;
                        #Response::build()
                            .status(#__status)
                            .merge(__response)
                            .ok()
                    })
                }

                #_catcher::StaticInfo {
                    name: ::core::stringify!(#user_catcher_fn_name),
                    code: #status_code,
                    handler: monomorphized_function,
                    type_id: #error_type,
                    location: (::core::file!(), ::core::line!(), ::core::column!()),
                }
            }

            #[doc(hidden)]
            pub fn into_catcher(self) -> #Catcher {
                self.into_info().into()
            }
        }
    })
}

pub fn catch_attribute(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream
) -> TokenStream {
    _catch(args, input).unwrap_or_else(|d| d.emit_as_item_tokens())
}
