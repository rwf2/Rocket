use std::ops::RangeBounds;

use devise::Diagnostic;
use proc_macro2::{Span, Literal};

// An experiment.
pub struct Diagnostics(Vec<Diagnostic>);

impl Diagnostics {
    pub fn new() -> Self {
        Diagnostics(vec![])
    }

    pub fn push(&mut self, diag: Diagnostic) {
        self.0.push(diag);
    }

    pub fn emit_head(self) -> Diagnostic {
        let mut iter = self.0.into_iter();
        let mut last = iter.next().expect("Diagnostic::emit_head empty");
        for diag in iter {
            // FIXME(diag: emit, can there be errors here?)
            last.emit_as_item_tokens();
            last = diag;
        }

        last
    }

    pub fn head_err_or<T>(self, ok: T) -> devise::Result<T> {
        match self.0.is_empty() {
            true => Ok(ok),
            false => Err(self.emit_head())
        }
    }
}

impl From<Diagnostic> for Diagnostics {
    fn from(diag: Diagnostic) -> Self {
        Diagnostics(vec![diag])
    }
}

impl From<Vec<Diagnostic>> for Diagnostics {
    fn from(diags: Vec<Diagnostic>) -> Self {
        Diagnostics(diags)
    }
}

pub struct StringLit(pub String, pub Literal);

impl StringLit {
    pub fn new<S: Into<String>>(string: S, span: Span) -> Self {
        let string = string.into();
        let mut lit = Literal::string(&string);
        lit.set_span(span);
        StringLit(string, lit)
    }

    pub fn span(&self) -> Span {
        self.1.span()
    }

    /// Attempt to obtain a subspan, or, failing that, produce the full span.
    /// This will create suboptimal diagnostics, but better than failing to
    /// build entirely.
    pub fn subspan<R: RangeBounds<usize>>(&self, range: R) -> Span {
        self.1.subspan(range).unwrap_or_else(|| self.span())
    }
}

impl syn::parse::Parse for StringLit {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let lit = input.parse::<syn::LitStr>()?;
        Ok(StringLit::new(lit.value(), lit.span()))
    }
}

impl devise::FromMeta for StringLit {
    fn from_meta(meta: &devise::MetaItem) -> devise::Result<Self> {
        Ok(StringLit::new(String::from_meta(meta)?, meta.value_span()))
    }
}

impl std::ops::Deref for StringLit {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}
