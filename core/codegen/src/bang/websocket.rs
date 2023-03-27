use devise::{Level, Diagnostic, Spanned};
use proc_macro2::TokenStream;
use syn::{parse::{Parse, ParseStream, discouraged::Speculative}, Pat};

pub fn _macro(input: proc_macro::TokenStream) -> devise::Result<TokenStream> {
    let closure: syn::ExprClosure = syn::parse(input)?;

    if closure.inputs.len() != 1 {
        return Err(Diagnostic::spanned(
            closure.inputs.span(),
            Level::Error,
            "rocket::response::websocket::CreateWebsocket! needs exactly one closure input"
        ));
    }

    if closure.capture.is_none() {
        return Err(
            Diagnostic::spanned(
                closure.span(),
                Level::Error,
                "rocket::response::websocket::CreateWebsocket! needs an closure that captures it's inputs"
            )
                .span_help(closure.or1_token.span(), "add the `move` keyword to the closure")
        );
    }

    let inp = closure.inputs.first();
    let body = closure.body;
    let capture = closure.capture;
    let tokens = quote!(
        Websocket::create(#capture |#inp| {
            ::std::boxed::Box::new(
                async #capture {
                    #body
                }
            )
        })
    );
    Ok(tokens)
}