use devise::ext::SpanDiagnosticExt;
use devise::{Diagnostic, FromMeta, MetaItem, Result, SpanWrapped, Spanned};
use proc_macro2::TokenStream;

use crate::attribute::param::{Dynamic, Guard};
use crate::attribute::Arguments;
use crate::proc_macro_ext::Diagnostics;
use crate::{http, http_codegen};

/// This structure represents the parsed `catch` attribute and associated items.
pub struct Attribute {
    /// The status associated with the code in the `#[catch(code)]` attribute.
    pub status: Option<http::Status>,
    /// The parameter to be used as the error type.
    pub error: Option<Guard>,
    /// All the other guards
    pub guards: Vec<Guard>,
    /// The function that was decorated with the `catch` attribute.
    pub function: syn::ItemFn,
}

/// We generate a full parser for the meta-item for great error messages.
#[derive(FromMeta)]
struct Meta {
    #[meta(naked)]
    code: Code,
    error: Option<SpanWrapped<Dynamic>>,
}

/// `Some` if there's a code, `None` if it's `default`.
#[derive(Debug)]
struct Code(Option<http::Status>);

impl FromMeta for Code {
    fn from_meta(meta: &MetaItem) -> Result<Self> {
        const HELP_MESSAGE: &str = "`#[catch]` expects a status code int or `default`: \
                         `#[catch(404)]` or `#[catch(default)]`";
        if usize::from_meta(meta).is_ok() {
            let status = http_codegen::Status::from_meta(meta).map_err(|diag|
                diag.help(HELP_MESSAGE))?;
            Ok(Code(Some(status.0)))
        } else if let MetaItem::Path(path) = meta {
            if path.is_ident("default") {
                Ok(Code(None))
            } else {
                Err(meta
                    .span()
                    .error("expected `default`")
                    .help(HELP_MESSAGE)
                 )
            }
        } else {
            let msg = format!("expected integer or `default`, found {}", meta.description());
            Err(meta
                .span()
                .error(msg)
                .help(HELP_MESSAGE)
            )
        }
    }
}

impl Attribute {
    pub fn parse(args: TokenStream, input: proc_macro::TokenStream) -> Result<Self> {
        let function: syn::ItemFn = syn::parse(input)
            .map_err(Diagnostic::from)
            .map_err(|diag| diag.help("`#[catch]` can only be used on functions"))?;

        let attr: MetaItem = syn::parse2(quote!(catch(#args)))?;
        let meta = Meta::from_meta(&attr)?;

        let mut diags = Diagnostics::new();
        let arguments = Arguments::from_signature(&function.sig, &mut diags);

        let error = meta
            .error
            .as_ref()
            .map(|e| arguments.map.get(&e.name).ok_or(e))
            .and_then(|e| e.map_err(|s| {
                let note = format!("expected argument named `{}` here", s.name);
                let diag = s.span()
                    .error("unused parameter")
                    .span_note(function.sig.inputs.span(), note);
                diags.push(diag)
            }).ok())
            .map(|(i, ty)| Guard {
                source: meta.error.clone().unwrap().value,
                fn_ident: i.clone(),
                ty: ty.clone(),
            });
        let guards = arguments.map
            .iter()
            .filter(|(name, _)| Some(*name) != meta.error.as_ref().map(|e| &e.name))
            .enumerate()
            .map(|(index, (name, (ident, ty)))| Guard {
                source: Dynamic { name: name.clone(), index, trailing: false },
                fn_ident: ident.clone(),
                ty: ty.clone(),
            })
            .collect();

        diags.head_err_or(Attribute { status: meta.code.0, error, guards, function })
    }
}
