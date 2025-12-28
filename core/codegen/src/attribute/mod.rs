use devise::{ext::{SpanDiagnosticExt as _, TypeExt as _}, Spanned};
use indexmap::IndexMap;
use proc_macro2::Span;
use syn::Signature;

use crate::{name::Name, proc_macro_ext::Diagnostics, syn_ext::FnArgExt as _};

type ArgumentMap = IndexMap<Name, (syn::Ident, syn::Type)>;

#[derive(Debug)]
pub struct Arguments {
    pub span: Span,
    pub map: ArgumentMap
}

impl Arguments {
    pub fn from_signature(sig: &Signature, diags: &mut Diagnostics) -> Self {
        let span = sig.paren_token.span.join();
        let mut arguments = Self { span, map: ArgumentMap::new() };
        for arg in &sig.inputs {
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
        arguments
    }
}

pub mod entry;
pub mod catch;
pub mod route;
pub mod param;
pub mod async_bound;
pub mod suppress;
