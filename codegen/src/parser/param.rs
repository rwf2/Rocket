use syntax::ast::Ident;
use syntax::ext::base::ExtCtxt;
use syntax::source_map::{Span, Spanned};

use utils::{span, SpanExt};

#[derive(Debug)]
pub enum Param {
    Single(Spanned<Ident>),
    Many(Spanned<Ident>),
}

impl Param {
    pub fn inner(&self) -> &Spanned<Ident> {
        match *self {
            Param::Single(ref ident) | Param::Many(ref ident) => ident,
        }
    }

    pub fn ident(&self) -> &Ident {
        match *self {
            Param::Single(ref ident) | Param::Many(ref ident) => &ident.node,
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
        ParamIter { ctxt: c, span: p, string: s }
    }
}

impl<'s, 'a, 'c> Iterator for ParamIter<'s, 'a, 'c> {
    type Item = Param;

    fn next(&mut self) -> Option<Param> {
        let err = |ecx: &ExtCtxt, sp: Span, msg: &str| {
            ecx.span_err(sp, msg);
            None
        };

        // Find the start and end indexes for the next parameter, if any.
        let (start, end) = match self.string.find('<') {
            Some(i) => match self.string.find('>') {
                Some(j) => (i, j),
                None => return err(self.ctxt, self.span, "malformed parameters")
            },
            _ => return None,
        };

        // Ensure we found a valid parameter.
        if end <= start {
            return err(self.ctxt, self.span, "malformed parameters");
        }

        // Calculate the parameter's ident.
        let full_param = &self.string[(start + 1)..end];
        let (is_many, param) = if full_param.ends_with("..") {
            (true, &full_param[..(full_param.len() - 2)])
        } else {
            (false, full_param)
        };

        let param_span = self.span.trim_left(start).shorten_to(end + 1);

        // Advance the string and span.
        self.string = &self.string[(end + 1)..];
        self.span = self.span.trim_left(end + 1);

        let spanned_ident = span(Ident::from_str(param), param_span);
        if is_many {
            Some(Param::Many(spanned_ident))
        } else {
            Some(Param::Single(spanned_ident))
        }
    }
}
