use utils::{self, ParserExt, SpanExt};

use syntax::codemap::{Spanned, Span};
use syntax::ext::base::ExtCtxt;
use syntax::symbol::LocalInternedString;
use syntax::ast::{self, Expr, Name, Ident, Path};
use syntax::parse::{SeqSep, PResult};
use syntax::parse::token::{DelimToken, Token};
use syntax::parse::parser::{Parser, PathStyle};
use syntax::print::pprust::ty_to_string;
use syntax::ptr::P;

use indexmap::IndexMap;

#[derive(Debug)]
enum Arg {
    Unnamed(P<Expr>),
    Named(Spanned<Ident>, P<Expr>),
}

#[derive(Debug)]
pub enum Args {
    Unnamed(Vec<P<Expr>>),
    Named(Vec<(Spanned<Ident>, P<Expr>)>),
}

#[derive(Debug)]
pub struct UriParams {
    pub mount_point: Option<Spanned<LocalInternedString>>,
    pub route_path: Path,
    pub arguments: Option<Spanned<Args>>,
}

#[derive(Debug)]
pub struct InternalUriParams {
    pub uri: Spanned<String>,
    pub fn_args: Vec<(ast::Ident, P<ast::Ty>)>,
    pub uri_params: UriParams,
}

impl Arg {
    fn is_named(&self) -> bool {
        match *self {
            Arg::Named(..) => true,
            Arg::Unnamed(_) => false,
        }
    }

    fn unnamed(self) -> P<Expr> {
        match self {
            Arg::Unnamed(expr) => expr,
            _ => panic!("Called Arg::unnamed() on an Arg::named!"),
        }
    }

    fn named(self) -> (Spanned<Ident>, P<Expr>) {
        match self {
            Arg::Named(ident, expr) => (ident, expr),
            _ => panic!("Called Arg::named() on an Arg::Unnamed!"),
        }
    }
}

impl UriParams {
    // Parses the mount point, if any, and route identifier.
    pub fn parse_prelude<'a>(
        ecx: &'a ExtCtxt,
        parser: &mut Parser<'a>
    ) -> PResult<'a, (Option<Spanned<LocalInternedString>>, Path)> {
        if parser.token == Token::Eof {
            return Err(ecx.struct_span_err(ecx.call_site(),
                "call to `uri!` cannot be empty"));
        }

        // Parse the mount point and suffixing ',', if any.
        let mount_point = match parser.parse_optional_str() {
            Some((symbol, _, _)) => {
                let string = symbol.as_str();
                let span = parser.prev_span;
                if string.contains('<') || !string.starts_with('/') {
                    let mut diag = ecx.struct_span_err(span, "invalid mount point");
                    diag.help("mount points must be static, absolute URIs: `/example`");
                    return Err(diag);
                }

                parser.expect(&Token::Comma)?;
                Some(span.wrap(string))
            }
            None => None,
        };

        // Parse the route identifier, which must always exist.
        let route_path = parser.parse_path(PathStyle::Mod)?;
        Ok((mount_point, route_path))
    }

    /// The Span to use when referring to all of the arguments.
    pub fn args_span(&self) -> Span {
        match self.arguments {
            Some(ref args) => args.span,
            None => self.route_path.span
        }
    }

    pub fn parse<'a>(
        ecx: &'a ExtCtxt,
        parser: &mut Parser<'a>
    ) -> PResult<'a, UriParams> {
        // Parse the mount point and suffixing ',', if any.
        let (mount_point, route_path) = Self::parse_prelude(ecx, parser)?;

        // If there are no arguments, finish early.
        if !parser.eat(&Token::Colon) {
            parser.expect(&Token::Eof)?;
            let arguments = None;
            return Ok(UriParams { mount_point, route_path, arguments, });
        }

        // Parse arguments.
        let mut args_span = parser.span;
        let comma = SeqSep::trailing_allowed(Token::Comma);
        let arguments = parser.parse_seq_to_end(&Token::Eof, comma, |parser| {
            let has_key = parser.look_ahead(1, |token| *token == Token::Eq);

            if has_key {
                let inner_ident = parser.parse_ident()?;
                let ident = parser.prev_span.wrap(inner_ident);
                parser.expect(&Token::Eq)?;

                let expr = parser.parse_expr()?;
                Ok(Arg::Named(ident, expr))
            } else {
                let expr = parser.parse_expr()?;
                Ok(Arg::Unnamed(expr))
            }
        })?;

        // Set the end of the args_span to be the end of the args.
        args_span = args_span.with_hi(parser.prev_span.hi());

        // A 'colon' was used but there are no arguments.
        if arguments.is_empty() {
            return Err(ecx.struct_span_err(parser.prev_span,
                                           "expected argument list after `:`"));
        }

        // Ensure that both types of arguments were not used at once.
        let (mut homogeneous_args, mut prev_named) = (true, None);
        for arg in arguments.iter() {
            match prev_named {
                Some(prev_named) => homogeneous_args = prev_named == arg.is_named(),
                None => prev_named = Some(arg.is_named()),
            }
        }

        if !homogeneous_args {
            return Err(ecx.struct_span_err(args_span,
                                           "named and unnamed parameters cannot be mixed"));
        }

        // Create the `Args` enum, which properly types one-kind-of-argument-ness.
        let args = if prev_named.unwrap() {
            Args::Named(arguments.into_iter().map(|arg| arg.named()).collect())
        } else {
            Args::Unnamed(arguments.into_iter().map(|arg| arg.unnamed()).collect())
        };

        let arguments = Some(args_span.wrap(args));
        Ok(UriParams { mount_point, route_path, arguments, })
    }
}

pub enum Validation {
    // Number expected, what we actually got.
    Unnamed(usize, usize),
    // (Missing, Extra, Duplicate)
    Named(Vec<Name>, Vec<Spanned<Ident>>, Vec<Spanned<Ident>>),
    // Everything is okay.
    Ok(Vec<P<Expr>>)
}

impl InternalUriParams {
    pub fn parse<'a>(
        ecx: &'a ExtCtxt,
        parser: &mut Parser<'a>,
    ) -> PResult<'a, InternalUriParams> {
        let uri_str = parser.parse_str_lit().map(|(s, _)| s.as_str().to_string())?;
        let uri = parser.prev_span.wrap(uri_str);
        parser.expect(&Token::Comma)?;

        let start = Token::OpenDelim(DelimToken::Paren);
        let end = Token::CloseDelim(DelimToken::Paren);
        let comma = SeqSep::trailing_allowed(Token::Comma);
        let fn_args = parser
            .parse_seq(&start, &end, comma, |parser| {
                let param = parser.parse_ident_inc_pat()?;
                parser.expect(&Token::Colon)?;
                let ty = utils::strip_ty_lifetimes(parser.parse_ty()?);
                Ok((param, ty))
            })?
            .node;

        parser.expect(&Token::Comma)?;
        let uri_params = UriParams::parse(ecx, parser)?;
        Ok(InternalUriParams { uri, fn_args, uri_params, })
    }

    pub fn fn_args_str(&self) -> String {
        self.fn_args.iter()
            .map(|&(ident, ref ty)| format!("{}: {}", ident.name, ty_to_string(&ty)))
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn validate(&self) -> Validation {
        let unnamed = |args: &Vec<P<Expr>>| -> Validation {
            let (expected, actual) = (self.fn_args.len(), args.len());
            if expected != actual { Validation::Unnamed(expected, actual) }
            else { Validation::Ok(args.clone()) }
        };

        match self.uri_params.arguments {
            None => unnamed(&vec![]),
            Some(Spanned { node: Args::Unnamed(ref args), .. }) => unnamed(args),
            Some(Spanned { node: Args::Named(ref args), .. }) => {
                let mut params: IndexMap<Name, Option<P<Expr>>> = self.fn_args.iter()
                    .map(|&(ident, _)| (ident.name, None))
                    .collect();

                let (mut extra, mut dup) = (vec![], vec![]);
                for &(ident, ref expr) in args {
                    match params.get_mut(&ident.node.name) {
                        Some(ref entry) if entry.is_some() => dup.push(ident),
                        Some(entry) => *entry = Some(expr.clone()),
                        None => extra.push(ident),
                    }
                }

                let (mut missing, mut exprs) = (vec![], vec![]);
                for (name, expr) in params.into_iter() {
                    match expr {
                        Some(expr) => exprs.push(expr),
                        None => missing.push(name)
                    }
                }

                if (extra.len() + dup.len() + missing.len()) == 0 {
                    Validation::Ok(exprs)
                } else {
                    Validation::Named(missing, extra, dup)
                }
            }
        }
    }

    pub fn uri_fmt_string(&self) -> String {
        self.uri.node
            .replace('<', "{fmt")
            .replace("..>", "}")
            .replace('>', "}")
    }
}
