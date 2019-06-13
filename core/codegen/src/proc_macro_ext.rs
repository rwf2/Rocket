use std::ops::RangeBounds;

use proc_macro::{Span, Diagnostic, Literal};

pub type PResult<T> = std::result::Result<T, Diagnostic>;

pub type DResult<T> = std::result::Result<T, Diagnostics>;

// An experiment.
pub struct Diagnostics(Vec<Diagnostic>);

impl Diagnostics {
    pub fn new() -> Self {
        Diagnostics(vec![])
    }

    pub fn push(&mut self, diag: Diagnostic) {
        self.0.push(diag);
    }

    pub fn join(mut self, mut diags: Diagnostics) -> Self {
        self.0.append(&mut diags.0);
        self
    }

    pub fn emit_head(self) -> Diagnostic {
        let mut iter = self.0.into_iter();
        let mut last = iter.next().expect("Diagnostic::emit_head empty");
        for diag in iter {
            last.emit();
            last = diag;
        }

        last
    }

    pub fn head_err_or<T>(self, ok: T) -> PResult<T> {
        match self.0.is_empty() {
            true => Ok(ok),
            false => Err(self.emit_head())
        }
    }

    pub fn err_or<T>(self, ok: T) -> DResult<T> {
        match self.0.is_empty() {
            true => Ok(ok),
            false => Err(self)
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

use std::ops::Deref;

pub struct StringLit(crate String, crate Literal);

impl Deref for StringLit {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

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
    /// This will create suboptimal diagnostics, but better than failing to build entirely.
    pub fn subspan<R: RangeBounds<usize>>(&self, range: R) -> Span {
        self.1.subspan(range).unwrap_or_else(|| self.span())
    }
}
