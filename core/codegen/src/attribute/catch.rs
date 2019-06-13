use proc_macro::{TokenStream, Span};
use devise::{syn, Spanned, Result, FromMeta};
use crate::proc_macro2::TokenStream as TokenStream2;

use crate::http_codegen::Status;
use crate::syn_ext::{syn_to_diag, IdentExt, ReturnTypeExt};
use self::syn::{Attribute, parse::Parser};
use crate::{CATCH_FN_PREFIX, CATCH_STRUCT_PREFIX};

/// The raw, parsed `#[catch(code)]` attribute.
#[derive(Debug, FromMeta)]
struct CatchAttribute {
    #[meta(naked)]
    status: Status
}

/// This structure represents the parsed `catch` attribute an associated items.
struct CatchParams {
    /// The status associated with the code in the `#[catch(code)]` attribute.
    status: Status,
    /// The function that was decorated with the `catch` attribute.
    function: syn::ItemFn,
}

fn parse_params(args: TokenStream2, input: TokenStream) -> Result<CatchParams> {
    let function: syn::ItemFn = syn::parse(input).map_err(syn_to_diag)
        .map_err(|diag| diag.help("`#[catch]` can only be used on functions"))?;

    let full_attr = quote!(#[catch(#args)]);
    let attrs = Attribute::parse_outer.parse2(full_attr).map_err(syn_to_diag)?;
    let attribute = match CatchAttribute::from_attrs("catch", &attrs) {
        Some(result) => result.map_err(|d| {
            d.help("`#[catch]` expects a single status integer, e.g.: #[catch(404)]")
        })?,
        None => return Err(Span::call_site().error("internal error: bad attribute"))
    };

    Ok(CatchParams { status: attribute.status, function })
}

pub fn _catch(args: TokenStream, input: TokenStream) -> Result<TokenStream> {
    // Parse and validate all of the user's input.
    let catch = parse_params(TokenStream2::from(args), input)?;

    // Gather everything we'll need to generate the catcher.
    let user_catcher_fn = &catch.function;
    let mut user_catcher_fn_name = catch.function.ident.clone();
    let generated_struct_name = user_catcher_fn_name.prepend(CATCH_STRUCT_PREFIX);
    let generated_fn_name = user_catcher_fn_name.prepend(CATCH_FN_PREFIX);
    let (vis, status) = (&catch.function.vis, &catch.status);
    let status_code = status.0.code;

    // Variables names we'll use and reuse.
    define_vars_and_mods!(req, catcher, response, Request, Response);

    // Determine the number of parameters that will be passed in.
    let (fn_sig, inputs) = match catch.function.decl.inputs.len() {
        0 => (quote!(fn() -> _), quote!()),
        1 => (quote!(fn(&#Request) -> _), quote!(#req)),
        _ => return Err(catch.function.decl.inputs.span()
                .error("invalid number of arguments: must be zero or one")
                .help("catchers may optionally take an argument of type `&Request`"))
    };

    // Set the span of the function name to point to inputs so that a later type
    // coercion failure points to the user's catcher's handler input.
    user_catcher_fn_name.set_span(catch.function.decl.inputs.span().into());

    // This ensures that "Responder not implemented" points to the return type.
    let return_type_span = catch.function.decl.output.ty()
        .map(|ty| ty.span().into())
        .unwrap_or(Span::call_site().into());

    let catcher_response = quote_spanned!(return_type_span => {
        // Emit this to force a type signature check.
        let #catcher: #fn_sig = #user_catcher_fn_name;
        ::rocket::response::Responder::respond_to(#catcher(#inputs), #req)?
    });

    // Generate the catcher, keeping the user's input around.
    Ok(quote! {
        #user_catcher_fn

        /// Rocket code generated wrapping catch function.
        #vis fn #generated_fn_name<'_b>(#req: &'_b #Request) -> #response::Result<'_b> {
            let __response = #catcher_response;
            #Response::build()
                .status(#status)
                .merge(__response)
                .ok()
        }

        /// Rocket code generated static catcher info.
        #[allow(non_upper_case_globals)]
        #vis static #generated_struct_name: ::rocket::StaticCatchInfo =
            ::rocket::StaticCatchInfo {
                code: #status_code,
                handler: #generated_fn_name,
            };
    }.into())
}

pub fn catch_attribute(args: TokenStream, input: TokenStream) -> TokenStream {
    _catch(args, input).unwrap_or_else(|d| { d.emit(); TokenStream::new() })
}
