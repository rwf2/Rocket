use syntax::codemap;
use syntax::parse::parser::{PathStyle, Parser};
use syntax::parse::{SeqSep, PResult};
use syntax::ast::{self, Path, StrStyle, Ident};
use syntax::parse::token;
use syntax::parse::token::Token::{Eof, Comma};
use syntax::symbol::Symbol;

pub trait ParserExt<'a> {
    // Parse a comma-seperated list of paths: `a::b, b::c`.
    fn parse_paths(&mut self) -> PResult<'a, Vec<Path>>;

    // Just like `parse_str` but takes into account interpolated expressions.
    fn parse_str_lit(&mut self) -> PResult<'a, (Symbol, StrStyle)>;

    // Like `parse_ident` but also looks for an `ident` in a `Pat`.
    fn parse_ident_inc_pat(&mut self) -> PResult<'a, Ident>;

    // Duplicates previously removed method in libsyntax
    fn parse_seq<T, F>(&mut self, bra: &token::Token, ket: &token::Token, sep: SeqSep, f: F)
        -> PResult<'a, codemap::Spanned<Vec<T>>> where F: FnMut(&mut Parser<'a>) -> PResult<'a, T>;
}

impl<'a> ParserExt<'a> for Parser<'a> {
    fn parse_paths(&mut self) -> PResult<'a, Vec<Path>> {
        self.parse_seq_to_end(&Eof,
                              SeqSep::trailing_allowed(Comma),
                              |p| p.parse_path(PathStyle::Mod))
    }

    fn parse_str_lit(&mut self) -> PResult<'a, (Symbol, StrStyle)> {
        self.parse_str()
            .or_else(|mut e| {
                let expr = self.parse_expr().map_err(|i| { e.cancel(); i })?;
                let string_lit = match expr.node {
                    ast::ExprKind::Lit(ref lit) => match lit.node {
                        ast::LitKind::Str(symbol, style) => (symbol, style),
                        _ => return Err(e)
                    }
                    _ => return Err(e)
                };

                e.cancel();
                Ok(string_lit)
            })
    }

    fn parse_ident_inc_pat(&mut self) -> PResult<'a, Ident> {
        self.parse_ident()
            .or_else(|mut e| {
                let pat = self.parse_pat().map_err(|i| { e.cancel(); i })?;
                let ident = match pat.node {
                    ast::PatKind::Ident(_, ident, _) => ident,
                    _ => return Err(e)
                };

                e.cancel();
                Ok(ident)
            })
    }

    // NB: Do not use this function unless you actually plan to place the
    // spanned list in the AST.
    fn parse_seq<T, F>(&mut self,
                           bra: &token::Token,
                           ket: &token::Token,
                           sep: SeqSep,
                           f: F)
                           -> PResult<'a, codemap::Spanned<Vec<T>>> where
        F: FnMut(&mut Parser<'a>) -> PResult<'a, T>,
    {
        let lo = self.span;
        self.expect(bra)?;
        let result = self.parse_seq_to_before_end(ket, sep, f)?;
        let hi = self.span;
        self.bump();
        Ok(codemap::respan(lo.to(hi), result))
    }
}
