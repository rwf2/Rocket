//! Extensions to `syn` types.

use devise::ext::SpanDiagnosticExt;
use devise::syn::{self, Ident, ext::IdentExt as _};

pub trait IdentExt {
    fn prepend(&self, string: &str) -> syn::Ident;
    fn append(&self, string: &str) -> syn::Ident;
}

impl IdentExt for syn::Ident {
    fn prepend(&self, string: &str) -> syn::Ident {
        syn::Ident::new(&format!("{}{}", string, self.unraw()), self.span())
    }

    fn append(&self, string: &str) -> syn::Ident {
        syn::Ident::new(&format!("{}{}", self, string), self.span())
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

pub trait TokenStreamExt {
    fn respanned(&self, span: crate::proc_macro2::Span) -> Self;
}

impl TokenStreamExt for crate::proc_macro2::TokenStream {
    fn respanned(&self, span: crate::proc_macro2::Span) -> Self {
        self.clone().into_iter().map(|mut token| {
            token.set_span(span);
            token
        }).collect()
    }
}

/// Represents the source of a name read by codegen, which may or may not be a
/// valid identifier. A `NameSource` is typically constructed indirectly via
/// FromMeta, or From<Ident> or directly from a string via `NameSource::new()`.
///
/// NameSource implements Hash, PartialEq, and Eq, and additionally PartialEq<S>
/// for all types `S: AsStr<str>`. These implementations all compare the value
/// of `name()` only.
#[derive(Debug, Clone)]
pub struct NameSource {
    name: String,
    ident: Option<Ident>,
}

impl NameSource {
    /// Creates a new `NameSource` from the string `name` and span `span`. If
    /// `name` is a valid ident, the ident is stored as well.
    pub fn new<S: AsRef<str>>(name: S, span: crate::proc_macro2::Span) -> Self {
        let name = name.as_ref();
        syn::parse_str::<Ident>(name)
            .map(|mut ident| { ident.set_span(span); ident })
            .map(|ident| NameSource::from(ident))
            .unwrap_or_else(|_| NameSource { name: name.into(), ident: None })
    }

    /// Returns the name as a string. Notably, if `self` was constructed from an
    /// Ident this method returns a name *without* an `r#` prefix.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the Ident corresponding to `self`, if any, otherwise panics. If
    /// `self` was constructed from an `Ident`, this never panics. Otherwise,
    /// panics if the string `self` was constructed from was not a valid ident.
    pub fn ident(&self) -> &Ident {
        self.ident.as_ref().expect("ident from namesource")
    }
}

impl devise::FromMeta for NameSource {
    fn from_meta(meta: devise::MetaItem<'_>) -> devise::Result<Self> {
        if let syn::Lit::Str(s) = meta.lit()? {
            return Ok(NameSource::new(s.value(), s.span()));
        }

        Err(meta.value_span().error("invalid value: expected string literal"))
    }
}

impl From<Ident> for NameSource {
    fn from(ident: Ident) -> Self {
        Self { name: ident.unraw().to_string(), ident: Some(ident), }
    }
}

impl std::hash::Hash for NameSource {
    fn hash<H: std::hash::Hasher>(&self, hasher: &mut H) {
        self.name().hash(hasher)
    }
}

impl PartialEq for NameSource {
    fn eq(&self, other: &Self) -> bool {
        self.name() == other.name()
    }
}

impl Eq for NameSource { }

impl<S: AsRef<str>> PartialEq<S> for NameSource {
    fn eq(&self, other: &S) -> bool {
        self.name == other.as_ref()
    }
}

impl std::fmt::Display for NameSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.name().fmt(f)
    }
}
