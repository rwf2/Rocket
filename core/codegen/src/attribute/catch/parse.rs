use devise::ext::{SpanDiagnosticExt, TypeExt};
use devise::{Diagnostic, FromMeta, MetaItem, Result, SpanWrapped, Spanned};
use proc_macro2::{Span, TokenStream, Ident};
use quote::ToTokens;

use crate::attribute::param::{Dynamic, Guard};
use crate::name::{ArgumentMap, Arguments, Name};
use crate::proc_macro_ext::Diagnostics;
use crate::syn_ext::FnArgExt;
use crate::{http, http_codegen};

/// This structure represents the parsed `catch` attribute and associated items.
pub struct Attribute {
    /// The status associated with the code in the `#[catch(code)]` attribute.
    pub status: Option<http::Status>,
    /// The function that was decorated with the `catch` attribute.
    pub function: syn::ItemFn,
    pub arguments: Arguments,
    pub error_guard: Option<ErrorGuard>,
    pub status_guard: Option<(Name, syn::Ident)>,
    pub request_guards: Vec<Guard>,
}

pub struct ErrorGuard {
    pub span: Span,
    pub name: Name,
    pub ident: syn::Ident,
    pub ty: syn::Type,
}

impl ErrorGuard {
    fn new(param: SpanWrapped<Dynamic>, args: &Arguments) -> Result<Self> {
        if let Some((ident, ty)) = args.map.get(&param.name) {
            match ty {
                syn::Type::Reference(syn::TypeReference { elem, .. }) => Ok(Self {
                    span: param.span(),
                    name: param.name.clone(),
                    ident: ident.clone(),
                    ty: elem.as_ref().clone(),
                }),
                ty => {
                    let msg = format!(
                        "Error argument must be a reference, found `{}`",
                        ty.to_token_stream()
                    );
                    let diag = param.span()
                        .error("invalid type")
                        .span_note(ty.span(), msg)
                        .help(format!("Perhaps use `&{}` instead", ty.to_token_stream()));
                    Err(diag)
                }
            }
        } else {
            let msg = format!("expected argument named `{}` here", param.name);
            let diag = param.span().error("unused parameter").span_note(args.span, msg);
            Err(diag)
        }
    }
}

fn status_guard(param: SpanWrapped<Dynamic>, args: &Arguments) -> Result<(Name, Ident)> {
    if let Some((ident, _)) = args.map.get(&param.name) {
        Ok((param.name.clone(), ident.clone()))
    } else {
        let msg = format!("expected argument named `{}` here", param.name);
        let diag = param.span().error("unused parameter").span_note(args.span, msg);
        Err(diag)
    }
}

/// We generate a full parser for the meta-item for great error messages.
#[derive(FromMeta)]
struct Meta {
    #[meta(naked)]
    code: Code,
    error: Option<SpanWrapped<Dynamic>>,
    status: Option<SpanWrapped<Dynamic>>,
}

/// `Some` if there's a code, `None` if it's `default`.
#[derive(Debug)]
struct Code(Option<http::Status>);

impl FromMeta for Code {
    fn from_meta(meta: &MetaItem) -> Result<Self> {
        if usize::from_meta(meta).is_ok() {
            let status = http_codegen::Status::from_meta(meta)?;
            Ok(Code(Some(status.0)))
        } else if let MetaItem::Path(path) = meta {
            if path.is_ident("default") {
                Ok(Code(None))
            } else {
                Err(meta.span().error("expected `default`"))
            }
        } else {
            let msg = format!("expected integer or `default`, found {}", meta.description());
            Err(meta.span().error(msg))
        }
    }
}

impl Attribute {
    pub fn parse(args: TokenStream, input: proc_macro::TokenStream) -> Result<Self> {
        let mut diags = Diagnostics::new();

        let function: syn::ItemFn = syn::parse(input)
            .map_err(Diagnostic::from)
            .map_err(|diag| diag.help("`#[catch]` can only be used on functions"))?;

        let attr: MetaItem = syn::parse2(quote!(catch(#args)))?;
        let attr = Meta::from_meta(&attr)
            .map(|meta| meta)
            .map_err(|diag| diag.help("`#[catch]` expects a status code int or `default`: \
                        `#[catch(404)]` or `#[catch(default)]`"))?;

        let span = function.sig.paren_token.span.join();
        let mut arguments = Arguments { map: ArgumentMap::new(), span };
        for arg in function.sig.inputs.iter() {
            if let Some((ident, ty)) = arg.typed() {
                let value = (ident.clone(), ty.with_stripped_lifetimes());
                arguments.map.insert(Name::from(ident), value);
            } else {
                let span = arg.span();
                let diag = if arg.wild().is_some() {
                    span.error("handler arguments must be named")
                        .help("to name an ignored handler argument, use `_name`")
                } else {
                    span.error("handler arguments must be of the form `ident: Type`")
                };

                diags.push(diag);
            }
        }
        // let mut error_guard = None;
        let error_guard = attr.error.clone()
            .map(|p| ErrorGuard::new(p, &arguments))
            .and_then(|p| p.map_err(|e| diags.push(e)).ok());
        let status_guard = attr.status.clone()
            .map(|n| status_guard(n, &arguments))
            .and_then(|p| p.map_err(|e| diags.push(e)).ok());
        let request_guards = arguments.map.iter()
            .filter(|(name, _)| {
                let mut all_other_guards = error_guard.iter()
                    .map(|g| &g.name)
                    .chain(status_guard.iter().map(|(n, _)| n));

                all_other_guards.all(|n| n != *name)
            })
            .enumerate()
            .map(|(index, (name, (ident, ty)))| Guard {
                source: Dynamic { index, name: name.clone(), trailing: false },
                fn_ident: ident.clone(),
                ty: ty.clone(),
            })
            .collect();

        diags.head_err_or(Attribute {
            status: attr.code.0,
            function,
            arguments,
            error_guard,
            status_guard,
            request_guards,
        })
    }
}
