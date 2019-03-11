//! Extensions to `syn` types.

use devise::syn;
use proc_macro::Diagnostic;

pub fn syn_to_diag(error: syn::parse::Error) -> Diagnostic {
    error.span().unstable().error(error.to_string())
}

pub trait IdentExt {
    fn prepend(&self, string: &str) -> syn::Ident;
}

impl IdentExt for syn::Ident {
    fn prepend(&self, string: &str) -> syn::Ident {
        syn::Ident::new(&format!("{}{}", string, self), self.span())
    }
}

pub trait ReturnTypeExt {
    fn ty(&self) -> Option<&syn::Type>;
}

impl ReturnTypeExt for syn::ReturnType {
    fn ty(&self) -> Option<&syn::Type> {
        match self {
            syn::ReturnType::Default => None,
            syn::ReturnType::Type(_, ty) => Some(ty),
        }
    }
}
