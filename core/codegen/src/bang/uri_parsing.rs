use proc_macro::Span;

use devise::{syn, Spanned};
use devise::proc_macro2::TokenStream as TokenStream2;
use devise::ext::TypeExt;
use quote::ToTokens;

use self::syn::{Expr, Ident, LitStr, Path, Token, Type};
use self::syn::parse::{self, Parse, ParseStream};
use self::syn::punctuated::Punctuated;

use http::{uri::Origin, ext::IntoOwned};
use indexmap::IndexMap;

#[derive(Debug)]
pub enum ArgExpr {
    Expr(Expr),
    Ignored(Token![_]),
}

#[derive(Debug)]
pub enum Arg {
    Unnamed(ArgExpr),
    Named(Ident, Token![=], ArgExpr),
}

#[derive(Debug)]
pub enum Args {
    Unnamed(Punctuated<Arg, Token![,]>),
    Named(Punctuated<Arg, Token![,]>),
}

// For an invocation that looks like:
//  uri!("/mount/point", this::route: e1, e2, e3);
//       ^-------------| ^----------| ^---------|
//           uri_params.mount_point |    uri_params.arguments
//                      uri_params.route_path
#[derive(Debug)]
pub struct UriParams {
    pub mount_point: Option<Origin<'static>>,
    pub route_path: Path,
    pub arguments: Args,
}

#[derive(Debug)]
pub struct FnArg {
    pub ident: Ident,
    pub ty: Type,
}

pub enum Validation<'a> {
    // Number expected, what we actually got.
    Unnamed(usize, usize),
    // (Missing, Extra, Duplicate)
    Named(Vec<&'a Ident>, Vec<&'a Ident>, Vec<&'a Ident>),
    // Everything is okay; here are the expressions in the route decl order.
    Ok(Vec<&'a ArgExpr>)
}

// This is invoked by Rocket itself. The `uri!` macro expands to a call to a
// route-specific macro which in-turn expands to a call to `internal_uri!`,
// passing along the user's parameters from the original `uri!` call. This is
// necessary so that we can converge the type information in the route (from the
// route-specific macro) with the user's parameters (by forwarding them to the
// internal_uri! call).
//
// `fn_args` are the URI arguments (excluding guards) from the original route's
// handler in the order they were declared in the URI (`<first>/<second>`).
// `uri` is the full URI used in the origin route's attribute.
//
//  internal_uri!("/<first>/<second>", (first: ty, second: ty), $($tt)*);
//                ^--------|---------  ^-----------|---------|  ^-----|
//                      route_uri               fn_args          uri_params
#[derive(Debug)]
pub struct InternalUriParams {
    pub route_uri: Origin<'static>,
    pub fn_args: Vec<FnArg>,
    pub uri_params: UriParams,
}

impl Parse for ArgExpr {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        if input.peek(Token![_]) {
            return Ok(ArgExpr::Ignored(input.parse::<Token![_]>()?));
        }

        input.parse::<Expr>().map(ArgExpr::Expr)
    }
}

impl Parse for Arg {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        let has_key = input.peek2(Token![=]);
        if has_key {
            let ident = input.parse::<Ident>()?;
            let eq_token = input.parse::<Token![=]>()?;
            let expr = input.parse::<ArgExpr>()?;
            Ok(Arg::Named(ident, eq_token, expr))
        } else {
            let expr = input.parse::<ArgExpr>()?;
            Ok(Arg::Unnamed(expr))
        }
    }
}

fn err<T, S: AsRef<str>>(span: Span, s: S) -> parse::Result<T> {
    Err(parse::Error::new(span.into(), s.as_ref()))
}

impl Parse for UriParams {
    // Parses the mount point, if any, route identifier, and arguments.
    fn parse(input: ParseStream) -> parse::Result<Self> {
        if input.is_empty() {
            return Err(input.error("call to `uri!` cannot be empty"));
        }

        // Parse the mount point and suffixing ',', if any.
        let mount_point = if input.peek(LitStr) {
            let string = input.parse::<LitStr>()?;
            let mount_point = Origin::parse_owned(string.value()).map_err(|_| {
                // TODO(proc_macro): use error, add example as a help
                parse::Error::new(string.span(), "invalid mount point; \
                    mount points must be static, absolute URIs: `/example`")
            })?;

            if !input.peek(Token![,]) && input.cursor().eof() {
                return err(string.span().unstable(), "unexpected end of input: \
                    expected ',' followed by route path");
            }

            input.parse::<Token![,]>()?;
            Some(mount_point)
        } else {
            None
        };

        // Parse the route identifier, which must always exist.
        let route_path = input.parse::<Path>()?;

        // If there are no arguments, finish early.
        if !input.peek(Token![:]) && input.cursor().eof() {
            let arguments = Args::Unnamed(Punctuated::new());
            return Ok(Self { mount_point, route_path, arguments });
        }

        // Parse arguments
        let colon = input.parse::<Token![:]>()?;
        let arguments: Punctuated<Arg, Token![,]> = input.parse_terminated(Arg::parse)?;

        // A 'colon' was used but there are no arguments.
        if arguments.is_empty() {
            return err(colon.span(), "expected argument list after `:`");
        }

        // Ensure that both types of arguments were not used at once.
        let (mut homogeneous_args, mut prev_named) = (true, None);
        for arg in &arguments {
            match prev_named {
                Some(prev_named) => homogeneous_args = prev_named == arg.is_named(),
                None => prev_named = Some(arg.is_named()),
            }
        }

        if !homogeneous_args {
            return err(arguments.span(), "named and unnamed parameters cannot be mixed");
        }

        // Create the `Args` enum, which properly record one-kind-of-argument-ness.
        let arguments = match prev_named {
            Some(true) => Args::Named(arguments),
            _ => Args::Unnamed(arguments)
        };

        Ok(Self { mount_point, route_path, arguments })
    }
}

impl Parse for FnArg {
    fn parse(input: ParseStream) -> parse::Result<FnArg> {
        let ident = input.parse::<Ident>()?;
        input.parse::<Token![:]>()?;
        let mut ty = input.parse::<Type>()?;
        ty.strip_lifetimes();
        Ok(FnArg { ident, ty })
    }
}

impl Parse for InternalUriParams {
    fn parse(input: ParseStream) -> parse::Result<InternalUriParams> {
        let route_uri_str = input.parse::<LitStr>()?;
        input.parse::<Token![,]>()?;

        // Validation should always succeed since this macro can only be called
        // if the route attribute succeeded, implying a valid route URI.
        let route_uri = Origin::parse_route(&route_uri_str.value())
            .map(|o| o.to_normalized().into_owned())
            .map_err(|_| input.error("internal error: invalid route URI"))?;

        let content;
        syn::parenthesized!(content in input);
        let fn_args: Punctuated<FnArg, Token![,]> = content.parse_terminated(FnArg::parse)?;
        let fn_args = fn_args.into_iter().collect();

        input.parse::<Token![,]>()?;
        let uri_params = input.parse::<UriParams>()?;
        Ok(InternalUriParams { route_uri, fn_args, uri_params })
    }
}

impl InternalUriParams {
    pub fn fn_args_str(&self) -> String {
        self.fn_args.iter()
            .map(|FnArg { ident, ty }| format!("{}: {}", ident, quote!(#ty).to_string().trim()))
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn validate(&self) -> Validation {
        let args = &self.uri_params.arguments;
        match args {
            Args::Unnamed(inner) => {
                let (expected, actual) = (self.fn_args.len(), inner.len());
                if expected != actual { Validation::Unnamed(expected, actual) }
                else { Validation::Ok(args.unnamed().unwrap().collect()) }
            },
            Args::Named(_) => {
                let mut params: IndexMap<&Ident, Option<&ArgExpr>> = self.fn_args.iter()
                    .map(|FnArg { ident, .. }| (ident, None))
                    .collect();

                let (mut extra, mut dup) = (vec![], vec![]);
                for (ident, expr) in args.named().unwrap() {
                    match params.get_mut(ident) {
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

impl UriParams {
    /// The Span to use when referring to all of the arguments.
    pub fn args_span(&self) -> Span {
        match self.arguments.num() {
            0 => self.route_path.span(),
            _ => self.arguments.span()
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

    fn named(&self) -> (&Ident, &ArgExpr) {
        match self {
            Arg::Named(ident, _, expr) => (ident, expr),
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

    fn named(&self) -> Option<impl Iterator<Item = (&Ident, &ArgExpr)>> {
        match self {
            Args::Named(args) => Some(args.iter().map(|arg| arg.named())),
            _ => None
        }
    }

    fn unnamed(&self) -> Option<impl Iterator<Item = &ArgExpr>> {
        match self {
            Args::Unnamed(args) => Some(args.iter().map(|arg| arg.unnamed())),
            _ => None
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

impl ToTokens for ArgExpr {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            ArgExpr::Expr(e) => e.to_tokens(tokens),
            ArgExpr::Ignored(e) => e.to_tokens(tokens)
        }
    }
}

impl ToTokens for Arg {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            Arg::Unnamed(e) => e.to_tokens(tokens),
            Arg::Named(ident, eq, expr) => tokens.extend(quote!(#ident #eq #expr)),
        }
    }
}

impl ToTokens for Args {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            Args::Unnamed(e) | Args::Named(e) => e.to_tokens(tokens)
        }
    }
}
