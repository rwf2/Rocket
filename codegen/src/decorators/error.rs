use ::{CATCH_STRUCT_PREFIX, CATCH_FN_PREFIX, CATCHER_ATTR};
use parser::ErrorParams;
use utils::*;

use syntax::codemap::{Span};
use syntax::ast::{MetaItem, Ident, TyKind};
use syntax::ext::base::{Annotatable, ExtCtxt};
use syntax::tokenstream::TokenTree;
use syntax::parse::token;

const ERR_PARAM: &'static str = "__err";
const REQ_PARAM: &'static str = "__req";

trait ErrorGenerateExt {
    fn generate_fn_arguments(&self, &ExtCtxt, Ident, Ident) -> Vec<TokenTree>;
}

impl ErrorGenerateExt for ErrorParams {
    fn generate_fn_arguments(&self, ecx: &ExtCtxt, err: Ident, req: Ident)
            -> Vec<TokenTree> {
        let arg_help = "error handlers can take either a `rocket::Error` or \
                      `rocket::Request` type, or both.";

        // Retrieve the params from the user's handler and check the number.
        let input_args = &self.annotated_fn.decl().inputs;
        if input_args.len() > 2 {
            let sp = self.annotated_fn.span();
            ecx.struct_span_err(sp, "error handlers can have at most 2 arguments")
                .help(arg_help).emit()
        }

        // (Imperfectly) inspect the types to figure which params to pass in.
        let args = input_args.iter().map(|arg| &arg.ty).filter_map(|ty| {
            match ty.node {
                TyKind::Rptr(..) => Some(req),
                TyKind::Path(..) => Some(err),
                _ => {
                    ecx.struct_span_err(ty.span, "unexpected error handler argument")
                        .help(arg_help).emit();
                    None
                }
            }
        }).collect::<Vec<_>>();

        sep_by_tok(ecx, &args, token::Comma)
    }
}

pub fn error_decorator(ecx: &mut ExtCtxt,
                       sp: Span,
                       meta_item: &MetaItem,
                       annotated: Annotatable)
    -> Vec<Annotatable>
{
    let mut output = Vec::new();

    // Parse the parameters from the error annotation.
    let error = ErrorParams::from(ecx, sp, meta_item, &annotated);

    // Get all of the information we learned from the attribute + function.
    let user_fn_name = error.annotated_fn.ident();
    let catch_fn_name = user_fn_name.prepend(CATCH_FN_PREFIX);
    let code = error.code.node;
    let err_ident = Ident::from_str(ERR_PARAM);
    let req_ident = Ident::from_str(REQ_PARAM);
    let fn_arguments = error.generate_fn_arguments(ecx, err_ident, req_ident);

    // Push the Rocket generated catch function.
    emit_item(&mut output, quote_item!(ecx,
        fn $catch_fn_name<'_b>($err_ident: ::rocket::Error,
                               $req_ident: &'_b ::rocket::Request)
                               -> ::rocket::response::Result<'_b> {
            let user_response = $user_fn_name($fn_arguments);
            let response = ::rocket::response::Responder::respond_to(user_response,
                                                                     $req_ident)?;
            let status = ::rocket::http::Status::raw($code);
            ::rocket::response::Response::build().status(status).merge(response).ok()
        }
    ).expect("catch function"));

    // Push the static catch info. This is what the errors! macro refers to.
    let struct_name = user_fn_name.prepend(CATCH_STRUCT_PREFIX);
    emit_item(&mut output, quote_item!(ecx,
        #[allow(non_upper_case_globals)]
        pub static $struct_name: ::rocket::StaticCatchInfo =
            ::rocket::StaticCatchInfo {
                code: $code,
                handler: $catch_fn_name
            };
    ).expect("catch info struct"));

    // Attach a `rocket_catcher` attribute to the user's function and emit it.
    let attr_name = Ident::from_str(CATCHER_ATTR);
    let catcher_attr = quote_attr!(ecx, #[$attr_name($struct_name)]);
    attach_and_emit(&mut output, catcher_attr, annotated);

    output
}
