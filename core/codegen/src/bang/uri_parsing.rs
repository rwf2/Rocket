use std::ops::Deref;

use indexmap::IndexMap;
use devise::{Spanned, ext::TypeExt};
use quote::{ToTokens, TokenStreamExt};
use syn::{Expr, Ident, LitStr, Path, Token, Type};
use syn::parse::{self, Parse, ParseStream, Parser};
use syn::punctuated::Punctuated;
use proc_macro2::{TokenStream, TokenTree, Span};
use rocket_http::uri::{Error, Reference};

use crate::http::uri::{Uri, Origin, Absolute, fmt};
use crate::http::ext::IntoOwned;
use crate::proc_macro_ext::StringLit;
use crate::attribute::param::{Parameter, Dynamic};
use crate::name::Name;

// TODO(diag): Use 'Diagnostic' in place of syn::Error.

#[derive(Debug)]
pub enum ArgExpr {
    Expr(Expr),
    Ignored(Token![_]),
}

#[derive(Debug)]
pub enum Arg {
    Unnamed(ArgExpr),
    Named(Name, Ident, Token![=], ArgExpr),
}

#[derive(Debug)]
pub enum Args {
    Unnamed(Punctuated<Arg, Token![,]>),
    Named(Punctuated<Arg, Token![,]>),
}

/// A string literal parsed as a URI.
#[derive(Debug)]
pub struct UriLit(Uri<'static>, Span);

/// An expression in a URI slot (prefix, suffix, or literal).
#[derive(Debug)]
pub enum UriExpr {
    /// A string literal parsed as a URI.
    Uri(UriLit),
    /// An expression that will be typechecked to be some URI kind.
    Expr(Expr),
}

/// See `UriMacro` for what each field represents.
#[derive(Debug)]
pub struct RouteInvocation {
    pub path: Path,
    pub args: Args,
}

/// See `UriMacro` for what each field represents.
#[derive(Debug)]
pub struct RoutedUri {
    pub prefix: Option<UriExpr>,
    pub route: RouteInvocation,
    pub suffix: Option<UriExpr>,
}

// The macro can be invoked with 1, 2, or 3 arguments.
//
// As a `Literal`, with a single argument:
//  uri!("/mount/point");
//       ^-------------|
//                 literal.0
//
// As `Routed`, with 1, 2, or 3 arguments: prefix/suffix optional.
//  uri!("/mount/point", this::route(e1, e2, e3), "?some#suffix");
//       ^-------------| ^---------|^----------|  |-----|------|
//              routed.prefix      |           |   routed.suffix
//                                 |   route.route.args
//                        routed.route.path
#[derive(Debug)]
pub enum UriMacro {
    Literal(UriLit),
    Routed(RoutedUri),
}

#[derive(Debug)]
pub enum Validation<'a> {
    // Parameters that were ignored in a named argument setting.
    NamedIgnored(Vec<&'a Dynamic>),
    // Number expected, what we actually got.
    Unnamed(usize, usize),
    // (Missing, Extra, Duplicate)
    Named(Vec<&'a Name>, Vec<&'a Ident>, Vec<&'a Ident>),
    // Everything is okay; here are the expressions in the route decl order.
    Ok(Vec<&'a ArgExpr>)
}

// This is invoked by Rocket itself. The `uri!` macro expands to a call to a
// route-specific macro which in-turn expands to a call to `internal_uri!`,
// passing along the user's invocation (`uri_mac`) from the original `uri!`
// call. This is necessary so that we can converge the type information in the
// route (from the route-specific macro) with the user's parameters (by
// forwarding them to the internal_uri! call).
//
// `fn_args` are the URI arguments (excluding request guards) from the original
// handler in the order they were declared in the URI (`<first>/<second>`).
// `route_uri` is the full route URI itself.
//
// The syntax of `uri_mac` is that of `UriMacro`.
//
//  internal_uri!("/<one>/<_>?lang=en&<two>", (one: ty, two: ty), $($tt)*);
//                ^----/----^ ^-----\-----^    ^-------/------^   ^-----|
//               path_params    query_params       fn_args          uri_mac
//                ^------ route_uri ------^
#[derive(Debug)]
pub struct InternalUriParams {
    pub route_uri: Origin<'static>,
    pub path_params: Vec<Parameter>,
    pub query_params: Vec<Parameter>,
    pub fn_args: Vec<FnArg>,
    pub uri_mac: RoutedUri,
}

#[derive(Debug)]
pub struct FnArg {
    pub ident: Ident,
    pub ty: Type,
}

fn err<T, S: AsRef<str>>(span: Span, s: S) -> parse::Result<T> {
    Err(parse::Error::new(span, s.as_ref()))
}

impl Parse for ArgExpr {
    fn parse(input: ParseStream<'_>) -> parse::Result<Self> {
        if input.peek(Token![_]) {
            return Ok(ArgExpr::Ignored(input.parse::<Token![_]>()?));
        }

        input.parse::<Expr>().map(ArgExpr::Expr)
    }
}

impl Parse for Arg {
    fn parse(input: ParseStream<'_>) -> parse::Result<Self> {
        let has_key = input.peek2(Token![=]);
        if has_key {
            let ident = input.parse::<Ident>()?;
            let eq_token = input.parse::<Token![=]>()?;
            let expr = input.parse::<ArgExpr>()?;
            Ok(Arg::Named(Name::from(&ident), ident, eq_token, expr))
        } else {
            let expr = input.parse::<ArgExpr>()?;
            Ok(Arg::Unnamed(expr))
        }
    }
}

impl Parse for Args {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        // If there are no arguments, finish early.
        if input.cursor().eof() {
            return Ok(Args::Unnamed(Punctuated::new()));
        }

        // Parse arguments. Ensure both types of args were not used at once.
        let args: Punctuated<Arg, Token![,]> = input.parse_terminated(Arg::parse)?;
        let mut first_is_named = None;
        for arg in &args {
            if let Some(first_is_named) = first_is_named {
                if first_is_named != arg.is_named() {
                    return err(args.span(), "named and unnamed parameters cannot be mixed");
                }
            } else {
                first_is_named = Some(arg.is_named());
            }
        }

        // Create the `Args` enum, which properly record one-kind-of-argument-ness.
        match first_is_named {
            Some(true) => Ok(Args::Named(args)),
            _ => Ok(Args::Unnamed(args))
        }
    }
}

impl Parse for RouteInvocation {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let path = input.parse()?;
        let args = if input.peek(syn::token::Paren) {
            let args;
            syn::parenthesized!(args in input);
            args.parse()?
        } else {
            Args::Unnamed(Punctuated::new())
        };

        Ok(RouteInvocation { path, args })
    }
}

impl Parse for UriLit {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let string = input.parse::<StringLit>()?;
        let uri = match Uri::parse_any(&string) {
            Ok(uri) => uri.into_owned(),
            Err(e) => {
                let span = string.subspan(e.index() + 1..(e.index() + 2));
                return err(span, format!("invalid URI: {}", e));
            }
        };

        Ok(UriLit(uri, string.span()))
    }
}

impl UriMacro {
    fn unary(input: ParseStream<'_>) -> parse::Result<Self> {
        if input.peek(LitStr) {
            Ok(UriMacro::Literal(input.parse()?))
        } else {
            Ok(UriMacro::Routed(RoutedUri {
                prefix: None,
                route: input.parse()?,
                suffix: None,
            }))
        }
    }

    fn binary(prefix: TokenStream, middle: TokenStream) -> parse::Result<Self> {
        Ok(UriMacro::Routed(RoutedUri {
            prefix: UriExpr::parse_prefix.parse2(prefix)?,
            route: syn::parse2(middle)?,
            suffix: None,
        }))
    }

    fn ternary(prefix: TokenStream, mid: TokenStream, suffix: TokenStream) -> parse::Result<Self> {
        Ok(UriMacro::Routed(RoutedUri {
            prefix: UriExpr::parse_prefix.parse2(prefix)?,
            route: syn::parse2(mid)?,
            suffix: UriExpr::parse_suffix.parse2(suffix)?
        }))
    }
}

impl Parse for UriMacro {
    fn parse(input: ParseStream<'_>) -> parse::Result<Self> {
        use syn::buffer::Cursor;
        use parse::{StepCursor, Result};

        fn stream<'c>(cursor: StepCursor<'c, '_>) -> Result<(Option<TokenStream>, Cursor<'c>)> {
            let mut stream = TokenStream::new();
            let mut cursor = *cursor;
            while let Some((tt, next)) = cursor.token_tree() {
                cursor = next;
                match tt {
                    TokenTree::Punct(p) if p.as_char() == ',' => break,
                    _ =>  stream.append(tt)
                }
            }

            stream.is_empty()
                .then(|| Ok((None, cursor)))
                .unwrap_or(Ok((Some(stream), cursor)))
        }

        let mut args = vec![];
        while let Some(tokens) = input.step(stream)? {
            args.push(tokens);
        }

        let (arg_count, mut iter) = (args.len(), args.into_iter());
        let mut next = || iter.next().unwrap();
        match arg_count {
            0 => err(Span::call_site(), "expected at least 1 argument, found none"),
            1 => UriMacro::unary.parse2(next()),
            2 => UriMacro::binary(next(), next()),
            3 => UriMacro::ternary(next(), next(), next()),
            n => err(iter.nth(3).unwrap().span(),
                format!("expected 1, 2, or 3 arguments, found {}", n))
        }
    }
}

impl Parse for RoutedUri {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        match UriMacro::parse(input)? {
            UriMacro::Routed(route) => Ok(route),
            UriMacro::Literal(uri) => err(uri.span(), "expected route URI, found literal")
        }
    }
}

impl Parse for FnArg {
    fn parse(input: ParseStream<'_>) -> parse::Result<FnArg> {
        let ident = input.parse::<Ident>()?;
        input.parse::<Token![:]>()?;
        let mut ty = input.parse::<Type>()?;
        ty.strip_lifetimes();
        Ok(FnArg { ident, ty })
    }
}

impl Parse for InternalUriParams {
    fn parse(input: ParseStream<'_>) -> parse::Result<InternalUriParams> {
        let route_uri_str = input.parse::<StringLit>()?;
        input.parse::<Token![,]>()?;

        // Validation should always succeed since this macro can only be called
        // if the route attribute succeeded, implying a valid route URI.
        let route_uri = Origin::parse_route(&route_uri_str)
            .map(|o| o.into_normalized().into_owned())
            .map_err(|_| input.error("internal error: invalid route URI"))?;

        let content;
        syn::parenthesized!(content in input);
        let fn_args: Punctuated<FnArg, Token![,]> = content.parse_terminated(FnArg::parse)?;
        let fn_args = fn_args.into_iter().collect();

        input.parse::<Token![,]>()?;
        let uri_params = input.parse::<RoutedUri>()?;

        let span = route_uri_str.subspan(1..route_uri.path().len() + 1);
        let path_params = Parameter::parse_many::<fmt::Path>(route_uri.path().as_str(), span)
            .map(|p| p.expect("internal error: invalid path parameter"))
            .collect::<Vec<_>>();

        let query = route_uri.query();
        let query_params = query.map(|query| {
            let i = route_uri.path().len() + 2;
            let span = route_uri_str.subspan(i..(i + query.len()));
            Parameter::parse_many::<fmt::Query>(query.as_str(), span)
                .map(|p| p.expect("internal error: invalid query parameter"))
                .collect::<Vec<_>>()
        }).unwrap_or_default();

        Ok(InternalUriParams {
            route_uri,
            path_params,
            query_params,
            fn_args,
            uri_mac: uri_params
        })
    }
}

impl InternalUriParams {
    pub fn fn_args_str(&self) -> String {
        self.fn_args.iter()
            .map(|FnArg { ident, ty }| {
                let ty = ty.with_stripped_lifetimes();
                let ty_str = quote!(#ty).to_string();
                let ty_str: String = ty_str.chars().filter(|c| !c.is_whitespace()).collect();
                format!("{}: {}", ident, ty_str)
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn dynamic_path_params(&self) -> impl Iterator<Item = &Dynamic> + Clone {
        self.path_params.iter()
            .filter_map(|p| p.dynamic().or_else(|| p.ignored()))
    }

    pub fn dynamic_query_params(&self) -> impl Iterator<Item = &Dynamic> + Clone {
        self.query_params.iter().filter_map(|p| p.dynamic())
    }

    pub fn validate(&self) -> Validation<'_> {
        let args = &self.uri_mac.route.args;
        let all_params = self.dynamic_path_params().chain(self.dynamic_query_params());
        match args {
            Args::Unnamed(args) => {
                let (expected, actual) = (all_params.count(), args.len());
                let unnamed_args = args.iter().map(|arg| arg.unnamed());
                match expected == actual {
                    true => Validation::Ok(unnamed_args.collect()),
                    false => Validation::Unnamed(expected, actual)
                }
            },
            Args::Named(args) => {
                let ignored = all_params.clone().filter(|p| p.is_wild());
                if ignored.clone().count() > 0 {
                    return Validation::NamedIgnored(ignored.collect());
                }

                let mut params = all_params.map(|p| (&p.name, None))
                    .collect::<IndexMap<&Name, Option<&ArgExpr>>>();

                let (mut extra, mut dup) = (vec![], vec![]);
                let named_args = args.iter().map(|arg| arg.named());
                for (name, ident, expr) in named_args {
                    match params.get_mut(name) {
                        Some(ref entry) if entry.is_some() => dup.push(ident),
                        Some(entry) => *entry = Some(expr),
                        None => extra.push(ident),
                    }
                }

                let (mut missing, mut exprs) = (vec![], vec![]);
                for (name, expr) in params {
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
}

impl RoutedUri {
    /// The Span to use when referring to all of the arguments.
    pub fn args_span(&self) -> Span {
        match self.route.args.num() {
            0 => self.route.path.span(),
            _ => self.route.args.span()
        }
    }
}

impl Arg {
    fn is_named(&self) -> bool {
        match *self {
            Arg::Named(..) => true,
            _ => false
        }
    }

    fn unnamed(&self) -> &ArgExpr {
        match self {
            Arg::Unnamed(expr) => expr,
            _ => panic!("Called Arg::unnamed() on an Arg::named!"),
        }
    }

    fn named(&self) -> (&Name, &Ident, &ArgExpr) {
        match self {
            Arg::Named(name, ident, _, expr) => (name, ident, expr),
            _ => panic!("Called Arg::named() on an Arg::Unnamed!"),
        }
    }
}

impl Args {
    fn num(&self) -> usize {
        match self {
            Args::Named(inner) | Args::Unnamed(inner) => inner.len(),
        }
    }
}

impl ArgExpr {
    pub fn as_expr(&self) -> Option<&Expr> {
        match self {
            ArgExpr::Expr(expr) => Some(expr),
            _ => None
        }
    }

    pub fn unwrap_expr(&self) -> &Expr {
        match self {
            ArgExpr::Expr(expr) => expr,
            _ => panic!("Called ArgExpr::expr() on ArgExpr::Ignored!"),
        }
    }
}

fn uri_err<T>(lit: &StringLit, error: Error<'_>) -> parse::Result<T> {
    let span = lit.subspan(error.index() + 1..(error.index() + 2));
    err(span, format!("invalid URI: {}", error))
}

impl UriExpr {
    fn parse_prefix(input: ParseStream<'_>) -> syn::Result<Option<Self>> {
        if input.parse::<Token![_]>().is_ok() {
            return Ok(None);
        }

        if !input.peek(LitStr) {
            return input.parse::<Expr>().map(|e| Some(UriExpr::Expr(e)));
        }

        let lit = input.parse::<StringLit>()?;
        let uri = Uri::parse::<Origin<'_>>(&lit)
            .or_else(|e| Uri::parse::<Absolute<'_>>(&lit).map_err(|e2| (e, e2)))
            .map_err(|(e1, e2)| lit.starts_with('/').then(|| e1).unwrap_or(e2))
            .or_else(|e| uri_err(&lit, e))?;

        if matches!(&uri, Uri::Origin(o) if o.query().is_some())
            || matches!(&uri, Uri::Absolute(a) if a.query().is_some())
        {
            return err(lit.span(), "URI prefix cannot contain query part");
        }

        Ok(Some(UriExpr::Uri(UriLit(uri.into_owned(), lit.span()))))
    }

    fn parse_suffix(input: ParseStream<'_>) -> syn::Result<Option<Self>> {
        if input.parse::<Token![_]>().is_ok() {
            return Ok(None);
        }

        if !input.peek(LitStr) {
            return input.parse::<Expr>().map(|e| Some(UriExpr::Expr(e)));
        }

        let lit = input.parse::<StringLit>()?;
        let uri = Reference::parse(&lit).or_else(|e| uri_err(&lit, e))?;
        if uri.scheme().is_some() || uri.authority().is_some() || !uri.path().is_empty() {
            return err(lit.span(), "URI suffix must contain only query and/or fragment");
        }

        // This is a bit of finagling to get the types to match up how we'd
        // like. A URI like `?foo` will parse as a `Reference`, since that's
        // what it is. But if we left this as is, we'd convert Origins and
        // Absolutes to References on suffix appendage when we don't need to.
        // This is because anything + a Reference _must_ result in a Reference
        // since the resulting URI could have a fragment. Since here we know
        // that's not the case, we lie and say it's Absolute since an Absolute
        // can't contain a fragment, so an Origin + Absolute suffix is still an
        // Origin, and likewise for an Absolute.
        let uri = match uri.fragment() {
            None => {
                let query = uri.query().map(|q| q.as_str());
                Uri::Absolute(Absolute::const_new("", None, "", query))
            }
            Some(_) => Uri::Reference(uri)
        };

        Ok(Some(UriExpr::Uri(UriLit(uri.into_owned(), lit.span()))))
    }
}

impl Deref for UriLit {
    type Target = Uri<'static>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ToTokens for UriLit {
    fn to_tokens(&self, t: &mut TokenStream) {
        use crate::http_codegen::*;

        let (uri, span) = (&self.0, self.1);
        match uri {
            Uri::Origin(o) => Origin(o, span).to_tokens(t),
            Uri::Absolute(o) => Absolute(o, span).to_tokens(t),
            Uri::Authority(o) => Authority(o, span).to_tokens(t),
            Uri::Reference(r) => Reference(r, span).to_tokens(t),
            Uri::Asterisk(a) => Asterisk(*a, span).to_tokens(t),
        }
    }
}

impl ToTokens for UriExpr {
    fn to_tokens(&self, t: &mut TokenStream) {
        match self {
            UriExpr::Uri(uri) => uri.to_tokens(t),
            UriExpr::Expr(e) => e.to_tokens(t),
        }
    }
}

impl ToTokens for ArgExpr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            ArgExpr::Expr(e) => e.to_tokens(tokens),
            ArgExpr::Ignored(e) => e.to_tokens(tokens)
        }
    }
}

impl ToTokens for Arg {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Arg::Unnamed(e) => e.to_tokens(tokens),
            Arg::Named(_, ident, eq, expr) => {
                ident.to_tokens(tokens);
                eq.to_tokens(tokens);
                expr.to_tokens(tokens);
            }
        }
    }
}

impl ToTokens for Args {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Args::Unnamed(e) | Args::Named(e) => e.to_tokens(tokens)
        }
    }
}
