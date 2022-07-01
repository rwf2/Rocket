use devise::ext::SpanDiagnosticExt;
use devise::{MetaItem, Spanned, Result, FromMeta, Diagnostic};
use proc_macro2::TokenStream;

use crate::{http, http_codegen};

/// This structure represents the parsed `catch` attribute and associated items.
pub struct Attribute {
    /// The status associated with the code in the `#[catch(code)]` attribute.
    pub status: Option<http::Status>,
    /// The function that was decorated with the `catch` attribute.
    pub function: syn::ItemFn,
}

/// We generate a full parser for the meta-item for great error messages.
#[derive(FromMeta)]
struct Meta {
    #[meta(naked)]
    code: Code,
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
        let function: syn::ItemFn = syn::parse(input)
            .map_err(Diagnostic::from)
            .map_err(|diag| diag.help("`#[catch]` can only be used on functions"))?;

        let attr: MetaItem = syn::parse2(quote!(catch(#args)))?;
        let status = Meta::from_meta(&attr)
            .map(|meta| meta.code.0)
            .map_err(|diag| diag.help("`#[catch]` expects a status code int or `default`: \
                        `#[catch(404)]` or `#[catch(default)]`"))?;

        Ok(Attribute { status, function })
    }
}
