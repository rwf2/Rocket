use syntax::ast::Ident;
use syntax::ext::base::ExtCtxt;
use syntax::codemap::{Span, Spanned, BytePos};

use utils::{span, SpanExt, is_valid_ident};

#[derive(Debug)]
pub enum Param {
    Single(Spanned<Ident>),
    Many(Spanned<Ident>)
}

impl Param {
    pub fn inner(&self) -> &Spanned<Ident> {
        match *self {
            Param::Single(ref ident) | Param::Many(ref ident) => ident
        }
    }

    pub fn ident(&self) -> &Ident {
        match *self {
            Param::Single(ref ident) | Param::Many(ref ident) => &ident.node
        }
    }
}

pub struct ParamIter<'s, 'a, 'c: 'a> {
    ctxt: &'a ExtCtxt<'c>,
    span: Span,
    string: &'s str,
}

impl<'s, 'a, 'c: 'a> ParamIter<'s, 'a, 'c> {
    pub fn new(c: &'a ExtCtxt<'c>, s: &'s str, p: Span) -> ParamIter<'s, 'a, 'c> {
        ParamIter {
            ctxt: c,
            span: p,
            string: s,
        }
    }
}

impl<'s, 'a, 'c> Iterator for ParamIter<'s, 'a, 'c> {
    type Item = Param;

    fn next(&mut self) -> Option<Param> {
        let err = |ecx: &ExtCtxt, sp: Span, msg: &str| {
            ecx.span_err(sp,  msg);
            None
        };

        // Find the start and end indexes for the next parameter, if any.
        let (start, end) = match self.string.find('<') {
            Some(i) => match self.string.find('>') {
                Some(j) => (i, j),
                None => return err(self.ctxt, self.span, "malformed parameter list")
            },
            _ => return None,
        };

        // Ensure we found a valid parameter.
        if end <= start {
            return err(self.ctxt, self.span, "malformed parameter list");
        }

        // Calculate the parameter's ident.
        let full_param = &self.string[(start + 1)..end];
        let (is_many, param) = if full_param.ends_with("..") {
            (true, &full_param[..(full_param.len() - 2)])
        } else {
            (false, full_param)
        };

        let mut param_span = self.span;
        param_span.lo = self.span.lo + BytePos(start as u32);
        param_span.hi = self.span.lo + BytePos((end + 1) as u32);

        // Advance the string and span.
        self.string = &self.string[(end + 1)..];
        self.span.lo = self.span.lo + BytePos((end + 1) as u32);

        // Check for nonemptiness, that the characters are correct, and return.
        if param.is_empty() {
            err(self.ctxt, param_span, "parameter names cannot be empty")
        } else if !is_valid_ident(param) {
            err(self.ctxt, param_span, "parameter names must be valid identifiers")
        } else if param.starts_with('_') {
            err(self.ctxt, param_span, "parameters cannot be ignored")
        } else if is_many && !self.string.is_empty() {
            let sp = self.span.shorten_to(self.string.len());
            self.ctxt.struct_span_err(sp, "text after a trailing '..' param")
                     .span_note(param_span, "trailing param is here")
                     .emit();
            None
        } else {
            let spanned_ident = span(Ident::from_str(param), param_span);
            if is_many {
                Some(Param::Many(spanned_ident))
            } else {
                Some(Param::Single(spanned_ident))
            }
        }

    }
}
