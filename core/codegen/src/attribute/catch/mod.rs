mod parse;

use devise::{Result, Spanned};
use proc_macro2::{TokenStream, Span};

use crate::http_codegen::Optional;
use crate::syn_ext::{IdentExt, ReturnTypeExt};
use crate::exports::*;

use self::parse::ErrorGuard;

use super::param::Guard;

fn error_type(guard: &ErrorGuard) -> TokenStream {
    let ty = &guard.ty;
    quote! {
        (#_catcher::TypeId::of::<#ty>(), ::std::any::type_name::<#ty>())
    }
}

fn error_guard_decl(guard: &ErrorGuard) -> TokenStream {
    let (ident, ty) = (guard.ident.rocketized(), &guard.ty);
    quote_spanned! { ty.span() =>
        let #ident: &#ty = match #_catcher::downcast(__error_init.as_ref()) {
            Some(v) => v,
            None => return #_Result::Err((#__status, __error_init)),
        };
    }
}

fn request_guard_decl(guard: &Guard) -> TokenStream {
    let (ident, ty) = (guard.fn_ident.rocketized(), &guard.ty);
    quote_spanned! { ty.span() =>
        let #ident: #ty = match <#ty as #FromRequest>::from_request(#__req).await {
            #Outcome::Success(__v) => __v,
            #Outcome::Forward(__e) => {
                ::rocket::trace::info!(
                    name: "forward",
                    target: concat!("rocket::codegen::catch::", module_path!()),
                    parameter = stringify!(#ident),
                    type_name = stringify!(#ty),
                    status = __e.code,
                    "request guard forwarding; trying next catcher"
                );

                return #_Err((#__status, __error_init));
            },
            #[allow(unreachable_code)]
            #Outcome::Error((__c, __e)) => {
                ::rocket::trace::info!(
                    name: "failure",
                    target: concat!("rocket::codegen::catch::", module_path!()),
                    parameter = stringify!(#ident),
                    type_name = stringify!(#ty),
                    reason = %#display_hack!(&__e),
                    "request guard failed; forwarding to 500 handler"
                );

                return #_Err((#Status::InternalServerError, __error_init));
            }
        };
    }
}

pub fn _catch(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream
) -> Result<TokenStream> {
    // Parse and validate all of the user's input.
    let catch = parse::Attribute::parse(args.into(), input.into())?;

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

    let status_guard = catch.status_guard.as_ref().map(|(_, s)| {
        let ident = s.rocketized();
        quote! { let #ident = #__status; }
    });
    let error_guard = catch.error_guard.as_ref().map(error_guard_decl);
    let error_type = Optional(catch.error_guard.as_ref().map(error_type));
    let request_guards = catch.request_guards.iter().map(request_guard_decl);
    let parameter_names = catch.arguments.map.values()
        .map(|(ident, _)| ident.rocketized());

    // We append `.await` to the function call if this is `async`.
    let dot_await = catch.function.sig.asyncness
        .map(|a| quote_spanned!(a.span() => .await));

    let catcher_response = quote_spanned!(return_type_span => {
        let ___responder = #user_catcher_fn_name(#(#parameter_names),*) #dot_await;
        #_response::Responder::respond_to(___responder, #__req).map_err(|s| (s, __error_init))?
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
                    #__req: &'__r #Request<'_>,
                    __error_init: #ErasedError<'__r>,
                ) -> #_catcher::BoxFuture<'__r> {
                    #_Box::pin(async move {
                        #error_guard
                        #status_guard
                        #(#request_guards)*
                        let __response = #catcher_response;
                        #_Result::Ok(
                            #Response::build()
                                .status(#__status)
                                .merge(__response)
                                .finalize()
                        )
                    })
                }

                #_catcher::StaticInfo {
                    name: ::core::stringify!(#user_catcher_fn_name),
                    code: #status_code,
                    error_type: #error_type,
                    handler: monomorphized_function,
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
